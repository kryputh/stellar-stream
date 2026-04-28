#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token::Client as TokenClient, Address, Env,
    Map, String, Vec,
};

// ---------------------------------------------------------------------------
// Stream struct
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stream {
    pub sender: Address,
    pub recipient: Address,
    pub token: Address,
    pub total_amount: i128,
    pub claimed_amount: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub cliff_seconds: u64,
    pub canceled: bool,
    pub paused: bool,
    pub pause_started_at: Option<u64>,
    pub metadata: Option<Map<String, String>>,
}

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

#[contracttype]
enum DataKey {
    Admin,
    NextStreamId,
    Stream(u64),
    SplitChildren(u64),
    ChildToParent(u64),
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCreated {
    pub stream_id: u64,
    pub sender: Address,
    pub recipient: Address,
    pub token: Address,
    pub token_symbol: String,
    pub total_amount: i128,
    pub start_time: u64,
    pub end_time: u64,
    pub cliff_seconds: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamClaimed {
    pub stream_id: u64,
    pub recipient: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamCanceled {
    pub stream_id: u64,
    pub sender: Address,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamTransferred {
    pub stream_id: u64,
    pub old_recipient: Address,
    pub new_recipient: Address,
}

/// Emitted when an admin claws back tokens from a stream for compliance purposes.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClawbackExecuted {
    pub stream_id: u64,
    pub amount: i128,
    pub recipient: Address,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct StellarStreamContract;

#[contractimpl]
impl StellarStreamContract {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// One-time setup: stores the admin address used for clawback authorization.
    /// Panics if called a second time to prevent privilege escalation.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    // -----------------------------------------------------------------------
    // Stream creation
    // -----------------------------------------------------------------------

    pub fn create_stream(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        total_amount: i128,
        start_time: u64,
        end_time: u64,
        cliff_seconds: u64,
    ) -> u64 {
        sender.require_auth();

        if total_amount <= 0 {
            panic!("total_amount must be positive");
        }
        if end_time <= start_time {
            panic!("end_time must be greater than start_time");
        }

        let token_client = TokenClient::new(&env, &token);
        let sender_balance = token_client.balance(&sender);
        if sender_balance < total_amount {
            panic!("insufficient sender balance");
        }

        // Escrow: transfer total_amount from sender into this contract.
        let contract_address = env.current_contract_address();
        token_client.transfer(&sender, &contract_address, &total_amount);

        let mut next_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextStreamId)
            .unwrap_or(0);
        next_id += 1;

        let stream = Stream {
            sender: sender.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            total_amount,
            claimed_amount: 0,
            start_time,
            end_time,
            cliff_seconds,
            canceled: false,
            paused: false,
            pause_started_at: None,
            metadata: metadata.clone(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::NextStreamId, &next_id);
        env.storage()
            .persistent()
            .set(&DataKey::Stream(next_id), &stream);

        env.events().publish(
            (symbol_short!("Stream"), symbol_short!("Created")),
            StreamCreated {
                stream_id: next_id,
                sender,
                recipient,
                token: token.clone(),
                token_symbol: token_client.symbol(),
                total_amount,
                start_time,
                end_time,
                cliff_seconds,
            },
        );

        next_id
    }

    pub fn create_split_stream(
        env: Env,
        sender: Address,
        token: Address,
        total_amount: i128,
        start_time: u64,
        end_time: u64,
        recipients: Vec<(Address, i128)>,
    ) -> u64 {
        sender.require_auth();
        if total_amount <= 0 {
            panic!("total_amount must be positive");
        }
        if end_time <= start_time {
            panic!("end_time must be greater than start_time");
        }
        if recipients.is_empty() {
            panic!("recipients must not be empty");
        }

        let token_client = TokenClient::new(&env, &token);
        let sender_balance = token_client.balance(&sender);
        if sender_balance < total_amount {
            panic!("insufficient sender balance");
        }
        let contract_address = env.current_contract_address();
        token_client.transfer(&sender, &contract_address, &total_amount);

        let mut next_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextStreamId)
            .unwrap_or(0);
        let parent_stream_id = next_id + 1;
        next_id = parent_stream_id;

        let mut allocated_total = 0_i128;
        let mut child_ids = Vec::<u64>::new(&env);
        for recipient_allocation in recipients.iter() {
            let recipient = recipient_allocation.0.clone();
            let allocation = recipient_allocation.1;
            if allocation <= 0 {
                panic!("allocation must be positive");
            }
            allocated_total += allocation;

            next_id += 1;
            let child_stream_id = next_id;
            let child_stream = Stream {
                sender: sender.clone(),
                recipient: recipient.clone(),
                token: token.clone(),
                total_amount: allocation,
                claimed_amount: 0,
                start_time,
                end_time,
                canceled: false,
                paused: false,
                pause_started_at: None,
                metadata: None,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Stream(child_stream_id), &child_stream);
            env.storage()
                .persistent()
                .set(&DataKey::ChildToParent(child_stream_id), &parent_stream_id);
            child_ids.push_back(child_stream_id);

            env.events().publish(
                (symbol_short!("Stream"), symbol_short!("Created")),
                StreamCreated {
                    stream_id: child_stream_id,
                    sender: sender.clone(),
                    recipient,
                    token: token.clone(),
                    token_symbol: token_client.symbol(),
                    total_amount: allocation,
                    start_time,
                    end_time,
                    metadata: None,
                },
            );
        }

        if allocated_total != total_amount {
            panic!("allocations must equal total_amount");
        }

        env.storage()
            .persistent()
            .set(&DataKey::SplitChildren(parent_stream_id), &child_ids);
        env.storage()
            .persistent()
            .set(&DataKey::NextStreamId, &next_id);
        parent_stream_id
    }

    pub fn get_split_children(env: Env, parent_stream_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::SplitChildren(parent_stream_id))
            .unwrap_or_else(|| Vec::<u64>::new(&env))
    }

    pub fn get_stream(env: Env, stream_id: u64) -> Stream {
        read_stream(&env, stream_id)
    }

    pub fn get_next_stream_id(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::NextStreamId)
            .unwrap_or(0)
    }

    pub fn claimable(env: Env, stream_id: u64, at_time: u64) -> i128 {
        let stream = read_stream(&env, stream_id);
        let vested = vested_amount(&stream, at_time);
        let claimable = vested - stream.claimed_amount;
        if claimable < 0 { 0 } else { claimable }
    }

    // -----------------------------------------------------------------------
    // Claim
    // -----------------------------------------------------------------------

    pub fn claim(env: Env, stream_id: u64, recipient: Address, amount: i128) -> i128 {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let mut stream = read_stream(&env, stream_id);
        if stream.recipient != recipient {
            panic!("recipient mismatch");
        }
        recipient.require_auth();

        let now = env.ledger().timestamp();
        let claimable_now = Self::claimable(env.clone(), stream_id, now);

        if amount > claimable_now {
            panic!("amount exceeds claimable");
        }

        let token_client = TokenClient::new(&env, &stream.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &recipient, &amount);

        stream.claimed_amount += amount;
        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);

        env.events().publish(
            (symbol_short!("Stream"), symbol_short!("Claimed")),
            StreamClaimed { stream_id, recipient, amount },
        );

        amount
    }

    // -----------------------------------------------------------------------
    // Cancel
    // -----------------------------------------------------------------------

    pub fn cancel(env: Env, stream_id: u64, sender: Address) {
        let mut stream = read_stream(&env, stream_id);
        if stream.sender != sender {
            panic!("sender mismatch");
        }
        sender.require_auth();

        if stream.canceled {
            return;
        }

        let now = env.ledger().timestamp();
        stream.canceled = true;

        let vested = vested_amount(&stream, now);
        let sender_refund = stream.total_amount - vested;

        let min_end = if now > stream.start_time { now } else { stream.start_time };
        if min_end < stream.end_time {
            stream.end_time = min_end;
            stream.total_amount = vested;
        }

        if sender_refund > 0 {
            let token_client = TokenClient::new(&env, &stream.token);
            let contract_address = env.current_contract_address();
            token_client.transfer(&contract_address, &sender, &sender_refund);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);

        env.events().publish(
            (symbol_short!("Stream"), symbol_short!("Canceled")),
            StreamCanceled { stream_id, sender },
        );
    }

    pub fn transfer_stream(env: Env, stream_id: u64, new_recipient: Address) {
        let mut stream = read_stream(&env, stream_id);
        stream.recipient.require_auth();

        let old_recipient = stream.recipient.clone();
        stream.recipient = new_recipient.clone();

        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);

        env.events().publish(
            (symbol_short!("Stream"), symbol_short!("Transfer")),
            StreamTransferred {
                stream_id,
                old_recipient,
                new_recipient,
            },
        );
    }

    pub fn pause_stream(env: Env, stream_id: u64, sender: Address) {
        let mut stream = read_stream(&env, stream_id);
        if stream.sender != sender {
            panic!("sender mismatch");
        }
        sender.require_auth();
        if stream.canceled {
            panic!("stream canceled");
        }
        if stream.paused {
            panic!("stream already paused");
        }

        stream.paused = true;
        stream.pause_started_at = Some(env.ledger().timestamp());
        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);
    }

    pub fn resume_stream(env: Env, stream_id: u64, sender: Address) {
        let mut stream = read_stream(&env, stream_id);
        if stream.sender != sender {
            panic!("sender mismatch");
        }
        sender.require_auth();
        if !stream.paused {
            panic!("stream is not paused");
        }

        let pause_started_at = stream
            .pause_started_at
            .unwrap_or_else(|| panic!("pause timestamp missing"));
        let now = env.ledger().timestamp();
        let paused_duration = now.saturating_sub(pause_started_at);
        stream.start_time = stream.start_time.saturating_add(paused_duration);
        stream.end_time = stream.end_time.saturating_add(paused_duration);
        stream.paused = false;
        stream.pause_started_at = None;

        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);
    }

    // -----------------------------------------------------------------------
    // Clawback
    // -----------------------------------------------------------------------

    pub fn clawback(env: Env, stream_id: u64, amount: i128, admin: Address) -> i128 {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let admin_stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("contract not initialized"));
        if admin_stored != admin {
            panic!("unauthorized");
        }
        admin.require_auth();

        let mut stream = read_stream(&env, stream_id);
        let now = env.ledger().timestamp();
        let vested = vested_amount(&stream, now);
        let unclaimed_vested = vested - stream.claimed_amount;

        let actual_clawback = if amount > unclaimed_vested {
            unclaimed_vested
        } else {
            amount
        };

        if actual_clawback > 0 {
            let token_client = TokenClient::new(&env, &stream.token);
            let contract_address = env.current_contract_address();
            token_client.transfer(&contract_address, &admin, &actual_clawback);

            stream.claimed_amount += actual_clawback;
            env.storage()
                .persistent()
                .set(&DataKey::Stream(stream_id), &stream);

            env.events().publish(
                (symbol_short!("Stream"), symbol_short!("Clawback")),
                ClawbackExecuted {
                    stream_id,
                    amount: actual_clawback,
                    recipient: admin,
                },
            );
        }

        actual_clawback
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn read_stream(env: &Env, stream_id: u64) -> Stream {
    env.storage()
        .persistent()
        .get(&DataKey::Stream(stream_id))
        .unwrap_or_else(|| panic!("stream not found"))
}

fn vested_amount(stream: &Stream, at_time: u64) -> i128 {
    if at_time < stream.start_time.saturating_add(stream.cliff_seconds) {
        return 0;
    }

    if at_time <= stream.start_time {
        return 0;
    }

    let effective_time = if effective_at_time >= stream.end_time {
        stream.end_time
    } else {
        effective_at_time
    };

    let elapsed = effective_time - stream.start_time;
    let total_duration = stream.end_time - stream.start_time;

    if total_duration == 0 {
        return 0;
    }

    stream.total_amount * (elapsed as i128) / (total_duration as i128)
}

#[cfg(test)]
mod test;
