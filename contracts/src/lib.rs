#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, token::Client as TokenClient, Address, Env,
    String, Vec,
};

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
    pub canceled: bool,
    pub paused: bool,
    pub pause_started_at: Option<u64>,
}

#[contracttype]
enum DataKey {
    NextStreamId,
    Stream(u64),
    SplitChildren(u64),
    ChildToParent(u64),
}

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

#[contract]
pub struct StellarStreamContract;

#[contractimpl]
impl StellarStreamContract {
    pub fn create_stream(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        total_amount: i128,
        start_time: u64,
        end_time: u64,
    ) -> u64 {
        sender.require_auth();

        if total_amount <= 0 {
            panic!("total_amount must be positive");
        }
        if end_time <= start_time {
            panic!("end_time must be greater than start_time");
        }

        // checks sebder balance.
        let token_client = TokenClient::new(&env, &token);
        let sender_balance = token_client.balance(&sender);
        if sender_balance < total_amount {
            panic!("insufficient sender balance");
        }

        // escrow = transfer total_amount from sender into this contract
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
            canceled: false,
            paused: false,
            pause_started_at: None,
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
        if claimable < 0 {
            0
        } else {
            claimable
        }
    }

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

        // amount claimed cannot exceed vested amount
        if amount > claimable_now {
            panic!("amount exceeds claimable");
        }

        // transfer tokens from contract escrow to recipient
        let token_client = TokenClient::new(&env, &stream.token);
        let contract_address = env.current_contract_address();
        token_client.transfer(&contract_address, &recipient, &amount);

        // Update accounting after successful transfer
        stream.claimed_amount += amount;
        env.storage()
            .persistent()
            .set(&DataKey::Stream(stream_id), &stream);

        env.events().publish(
            (symbol_short!("Stream"), symbol_short!("Claimed")),
            StreamClaimed {
                stream_id,
                recipient,
                amount,
            },
        );

        amount
    }

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

        // compute vested BEFORE truncating end_time
        let vested = vested_amount(&stream, now);
        let sender_refund = stream.total_amount - vested;

        // truncate end_time so recipient can't claim past cancel point
        let min_end = if now > stream.start_time {
            now
        } else {
            stream.start_time
        };
        if min_end < stream.end_time {
            stream.end_time = min_end;
            // Adjust total_amount to match the vested amount at cancel time
            // This ensures that the vested calculation remains correct after truncation
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
}

fn read_stream(env: &Env, stream_id: u64) -> Stream {
    env.storage()
        .persistent()
        .get(&DataKey::Stream(stream_id))
        .unwrap_or_else(|| panic!("stream not found"))
}

fn vested_amount(stream: &Stream, at_time: u64) -> i128 {
    let mut effective_at_time = at_time;
    if stream.paused {
        if let Some(paused_at) = stream.pause_started_at {
            if effective_at_time > paused_at {
                effective_at_time = paused_at;
            }
        }
    }

    if effective_at_time <= stream.start_time {
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
