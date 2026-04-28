#![cfg(test)]
extern crate std;
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, Map, String, Vec, symbol_short,
};
use insta::assert_debug_snapshot as assert_snapshot;

fn create_token(env: &Env, admin: &Address) -> Address {
    let token_contract_id = env.register_stellar_asset_contract_v2(admin.clone());
    token_contract_id.address()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns a simple one-entry metadata map for use in tests.
fn make_metadata(env: &Env) -> Map<String, String> {
    let mut m = Map::new(env);
    m.set(
        String::from_str(env, "department"),
        String::from_str(env, "engineering"),
    );
    m
}

// ---------------------------------------------------------------------------
// Existing stream-lifecycle tests (metadata = None)
// ---------------------------------------------------------------------------

#[test]
fn test_get_next_stream_id() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    assert_eq!(client.get_next_stream_id(), 0);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &5000);
    client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000, &None);
    assert_eq!(client.get_next_stream_id(), 1);
    client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000, &None);
    assert_eq!(client.get_next_stream_id(), 2);
}

#[test]
fn test_claim_transfers_tokens_to_recipient() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    let claimed = client.claim(&stream_id, &recipient, &500);
    assert_eq!(claimed, 500);
    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&recipient), 500);
}

#[test]
fn test_claim_partial_then_full() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &recipient, &300);
    env.ledger().with_mut(|l| l.timestamp = 1000);
    client.claim(&stream_id, &recipient, &700);
    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&recipient), 1000);
}

#[test]
#[should_panic(expected = "amount exceeds claimable")]
fn test_claim_cannot_exceed_vested_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 250);
    client.claim(&stream_id, &recipient, &500);
}

#[test]
#[should_panic(expected = "amount exceeds claimable")]
fn test_claim_cannot_double_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &recipient, &500);
    client.claim(&stream_id, &recipient, &500);
}

#[test]
#[should_panic(expected = "recipient mismatch")]
fn test_claim_fails_with_wrong_recipient() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let wrong_recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &wrong_recipient, &500);
}

#[test]
#[should_panic(expected = "insufficient sender balance")]
fn test_create_stream_fails_with_insufficient_sender_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &100);
    client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
}

#[test]
fn test_claimable_before_stream_start_returns_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000, &None);
    assert_eq!(client.claimable(&stream_id, &999), 0);
    assert_eq!(client.claimable(&stream_id, &1000), 0);
}

#[test]
fn test_claimable_during_stream_is_linear() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    assert_eq!(client.claimable(&stream_id, &250), 250);
    assert_eq!(client.claimable(&stream_id, &500), 500);
    assert_eq!(client.claimable(&stream_id, &750), 750);
}

#[test]
fn test_claimable_accounts_for_already_claimed() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &recipient, &300);
    assert_eq!(client.claimable(&stream_id, &500), 200);
}

#[test]
fn test_claimable_after_stream_end_caps_at_total() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    assert_eq!(client.claimable(&stream_id, &1000), 1000);
    assert_eq!(client.claimable(&stream_id, &9999), 1000);
}

#[test]
fn test_cancel_refunds_unclaimed_to_sender() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.cancel(&stream_id, &sender);
    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 500);
}

#[test]
fn test_cancel_marks_stream_as_canceled() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    client.cancel(&stream_id, &sender);
    let stream = client.get_stream(&stream_id);
    assert!(stream.canceled);
}

#[test]
fn test_cancel_idempotent_double_cancel_does_not_panic() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    client.cancel(&stream_id, &sender);
    client.cancel(&stream_id, &sender);
}

#[test]
fn test_cancel_recipient_cannot_claim_beyond_vested_at_cancel_time() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.cancel(&stream_id, &sender);
    client.claim(&stream_id, &recipient, &500);
    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&recipient), 500);
}

#[test]
#[should_panic(expected = "sender mismatch")]
fn test_cancel_fails_with_wrong_sender() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let wrong_sender = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    client.cancel(&stream_id, &wrong_sender);
}

#[test]
#[should_panic(expected = "amount must be positive")]
fn test_claim_zero_amount_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &recipient, &0);
}

#[test]
#[should_panic(expected = "amount exceeds claimable")]
fn test_claim_before_stream_start_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000, &None);
    client.claim(&stream_id, &recipient, &1);
}

#[test]
#[should_panic(expected = "stream not found")]
fn test_claim_nonexistent_stream_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);
    client.claim(&999, &recipient, &100);
}

#[test]
#[should_panic(expected = "total_amount must be positive")]
fn test_create_stream_zero_amount_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    client.create_stream(&sender, &recipient, &token, &0, &0, &1000, &None);
}

#[test]
#[should_panic(expected = "end_time must be greater than start_time")]
fn test_create_stream_invalid_time_range_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    client.create_stream(&sender, &recipient, &token, &1000, &1000, &1000, &None);
}

#[test]
fn test_event_emissions() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    let last_event = env.events().all().last().unwrap();

    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("Stream"), symbol_short!("Created")).into_val(&env)
    );

    let event_data: StreamCreated = last_event.2.into_val(&env);
    let expected_symbol = token::Client::new(&env, &token).symbol();
    assert_eq!(
        event_data,
        StreamCreated {
            stream_id: 1,
            sender: sender.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            token_symbol: expected_symbol,
            total_amount: 1000,
            start_time: 0,
            end_time: 1000,
            metadata: None,
        }
    );

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &recipient, &500);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("Stream"), symbol_short!("Claimed")).into_val(&env)
    );

    let event_data: StreamClaimed = last_event.2.into_val(&env);
    assert_eq!(
        event_data,
        StreamClaimed {
            stream_id,
            recipient: recipient.clone(),
            amount: 500,
        }
    );

    client.cancel(&stream_id, &sender);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("Stream"), symbol_short!("Canceled")).into_val(&env)
    );

    let event_data: StreamCanceled = last_event.2.into_val(&env);
    assert_eq!(
        event_data,
        StreamCanceled {
            stream_id,
            sender: sender.clone(),
        }
    );
}

#[test]
fn test_stream_created_snapshot() {
    let env = Env::default();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    let event = StreamCreated {
        stream_id: 1,
        sender: sender.clone(),
        recipient: recipient.clone(),
        token: token.clone(),
        token_symbol: soroban_sdk::String::from_str(&env, "TEST"),
        total_amount: 1000,
        start_time: 100,
        end_time: 200,
        metadata: None,
    };

    assert_snapshot!("stream_created_event", event);
}

#[test]
fn test_create_split_stream_creates_child_streams_and_links() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient_a = Address::generate(&env);
    let recipient_b = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let mut recipients = Vec::new(&env);
    recipients.push_back((recipient_a.clone(), 400));
    recipients.push_back((recipient_b.clone(), 600));

    let parent_id = client.create_split_stream(&sender, &token, &1000, &0, &1000, &recipients);
    let children = client.get_split_children(&parent_id);

    assert_eq!(children.len(), 2);
    let child_a_id = children.get(0).unwrap();
    let child_b_id = children.get(1).unwrap();

    let child_a = client.get_stream(&child_a_id);
    let child_b = client.get_stream(&child_b_id);
    assert_eq!(child_a.recipient, recipient_a);
    assert_eq!(child_a.total_amount, 400);
    assert_eq!(child_b.recipient, recipient_b);
    assert_eq!(child_b.total_amount, 600);
}

#[test]
fn test_split_stream_claim_and_cancel_work_per_substream() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient_a = Address::generate(&env);
    let recipient_b = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let token_client = token::Client::new(&env, &token);

    let mut recipients = Vec::new(&env);
    recipients.push_back((recipient_a.clone(), 400));
    recipients.push_back((recipient_b.clone(), 600));

    let parent_id = client.create_split_stream(&sender, &token, &1000, &0, &1000, &recipients);
    let children = client.get_split_children(&parent_id);
    let child_a_id = children.get(0).unwrap();
    let child_b_id = children.get(1).unwrap();

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&child_a_id, &recipient_a, &200);
    client.cancel(&child_b_id, &sender);

    assert_eq!(token_client.balance(&recipient_a), 200);
    assert_eq!(token_client.balance(&sender), 300);
    assert_eq!(client.claimable(&child_b_id, &1000), 300);
}

#[test]
fn test_pause_resume_freezes_vesting_and_extends_end_time() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);
    env.ledger().with_mut(|l| l.timestamp = 300);
    client.pause_stream(&stream_id, &sender);
    assert_eq!(client.claimable(&stream_id, &450), 300);

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.resume_stream(&stream_id, &sender);

    assert_eq!(client.claimable(&stream_id, &700), 500);
    assert_eq!(client.claimable(&stream_id, &1200), 1000);
}

#[test]
fn test_vested_amount_fuzz_invariants() {
    let env = Env::default();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);

    let stream = Stream {
        sender,
        recipient,
        token,
        total_amount: 1_000_000,
        claimed_amount: 0,
        start_time: 100,
        end_time: 10_100,
        canceled: false,
        paused: false,
        pause_started_at: None,
        metadata: None,
    };

    let mut seed: u64 = 0xDEADBEEFCAFEBABE;
    for _ in 0..2048 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let at_time = seed % 20_000;
        let vested = vested_amount(&stream, at_time);
        assert!(vested <= stream.total_amount);
        assert!(vested >= 0);
        if at_time <= stream.start_time {
            assert_eq!(vested, 0);
        }
        if at_time >= stream.end_time {
            assert_eq!(vested, stream.total_amount);
        }
    }
}

#[test]
fn test_claimable_at_start_time() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000, &None);
    assert_eq!(client.claimable(&stream_id, &1000), 0);
}

#[test]
fn test_claimable_at_end_time() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000, &None);
    assert_eq!(client.claimable(&stream_id, &2000), 1000);
}

#[test]
fn test_claimable_after_end_time() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000, &None);
    assert_eq!(client.claimable(&stream_id, &2100), 1000);
}

// -----------------------------------------------------------------
// CANCEL BEFORE STREAM START
// -----------------------------------------------------------------

#[test]
fn test_cancel_before_start_refunds_full_amount_to_sender() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &500, &1500, &None);

    env.ledger().with_mut(|l| l.timestamp = 0);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 1000);
    assert_eq!(token_client.balance(&recipient), 0);
}

#[test]
fn test_cancel_before_start_recipient_claimable_is_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &500, &1500, &None);

    env.ledger().with_mut(|l| l.timestamp = 0);
    client.cancel(&stream_id, &sender);

    assert_eq!(client.claimable(&stream_id, &1500), 0);
    assert_eq!(client.claimable(&stream_id, &9999), 0);
}

#[test]
#[should_panic(expected = "amount exceeds claimable")]
fn test_cancel_before_start_claim_attempt_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &500, &1500, &None);

    env.ledger().with_mut(|l| l.timestamp = 0);
    client.cancel(&stream_id, &sender);

    env.ledger().with_mut(|l| l.timestamp = 2000);
    client.claim(&stream_id, &recipient, &1);
}

// -----------------------------------------------------------------
// CANCEL MID-STREAM
// -----------------------------------------------------------------

#[test]
fn test_cancel_at_quarter_vesting() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 250);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 750);

    client.claim(&stream_id, &recipient, &250);
    assert_eq!(token_client.balance(&recipient), 250);
}

#[test]
fn test_cancel_midstream_no_prior_claims_splits_correctly() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 500);

    client.claim(&stream_id, &recipient, &500);
    assert_eq!(token_client.balance(&recipient), 500);

    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient),
        1000
    );
}

#[test]
fn test_cancel_midstream_non_round_amounts() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1200);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1200, &0, &1200, &None);

    env.ledger().with_mut(|l| l.timestamp = 900);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 300);
    assert_eq!(client.claimable(&stream_id, &900), 900);
}

#[test]
fn test_cancel_midstream_after_partial_claim_correct_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 400);
    client.claim(&stream_id, &recipient, &200);

    env.ledger().with_mut(|l| l.timestamp = 600);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 400);

    assert_eq!(client.claimable(&stream_id, &600), 400);
    client.claim(&stream_id, &recipient, &400);
    assert_eq!(token_client.balance(&recipient), 600);

    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient),
        1000
    );
}

#[test]
#[should_panic(expected = "amount exceeds claimable")]
fn test_cancel_midstream_recipient_cannot_claim_past_cancel_point() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.cancel(&stream_id, &sender);

    env.ledger().with_mut(|l| l.timestamp = 2000);
    client.claim(&stream_id, &recipient, &501);
}

// -----------------------------------------------------------------
// CANCEL AFTER FULL VESTING
// -----------------------------------------------------------------

#[test]
fn test_cancel_at_end_time_sender_gets_zero_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 1000);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 0);
}

#[test]
fn test_cancel_after_end_time_sender_gets_zero_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 9999);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 0);
}

#[test]
fn test_cancel_after_full_vesting_recipient_can_claim_all() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 9999);
    client.cancel(&stream_id, &sender);

    assert_eq!(client.claimable(&stream_id, &9999), 1000);
    client.claim(&stream_id, &recipient, &1000);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&recipient), 1000);
}

#[test]
fn test_cancel_after_full_vesting_with_partial_prior_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000, &None);

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &recipient, &300);

    env.ledger().with_mut(|l| l.timestamp = 1500);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 0);

    assert_eq!(client.claimable(&stream_id, &1500), 700);
    client.claim(&stream_id, &recipient, &700);
    assert_eq!(token_client.balance(&recipient), 1000);
}

// -----------------------------------------------------------------
// CONSERVATION INVARIANT
// -----------------------------------------------------------------

#[test]
fn test_cancel_token_conservation_across_timings() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &3600);

    let token_client = token::Client::new(&env, &token);

    let a = client.create_stream(&sender, &recipient, &token, &1200, &500, &1700, &None);
    env.ledger().with_mut(|l| l.timestamp = 100);
    let sender_before_a = token_client.balance(&sender);
    let recipient_before_a = token_client.balance(&recipient);
    client.cancel(&a, &sender);
    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient)
            - sender_before_a
            - recipient_before_a,
        1200
    );

    env.ledger().with_mut(|l| l.timestamp = 0);
    let b = client.create_stream(&sender, &recipient, &token, &1200, &0, &1200, &None);
    env.ledger().with_mut(|l| l.timestamp = 600);
    let sender_before_b = token_client.balance(&sender);
    let recipient_before_b = token_client.balance(&recipient);
    client.cancel(&b, &sender);
    client.claim(&b, &recipient, &600);
    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient)
            - sender_before_b
            - recipient_before_b,
        1200
    );

    env.ledger().with_mut(|l| l.timestamp = 0);
    let c = client.create_stream(&sender, &recipient, &token, &1200, &0, &1200, &None);
    env.ledger().with_mut(|l| l.timestamp = 9999);
    let sender_before_c = token_client.balance(&sender);
    let recipient_before_c = token_client.balance(&recipient);
    client.cancel(&c, &sender);
    client.claim(&c, &recipient, &1200);
    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient)
            - sender_before_c
            - recipient_before_c,
        1200
    );
}

// =============================================================================
// #121 — STREAM METADATA TESTS
// =============================================================================

/// create_stream with Some(metadata) persists and returns it via get_stream.
#[test]
fn test_create_stream_with_metadata_stored_in_stream() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let meta = make_metadata(&env);
    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000,
        &Some(meta.clone()),
    );

    let stream = client.get_stream(&stream_id);
    assert_eq!(stream.metadata, Some(meta));
}

/// None metadata is preserved as None in get_stream.
#[test]
fn test_create_stream_without_metadata_stores_none() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );

    let stream = client.get_stream(&stream_id);
    assert!(stream.metadata.is_none());
}

/// StreamCreated event carries the metadata supplied at creation.
#[test]
fn test_metadata_included_in_stream_created_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let meta = make_metadata(&env);
    client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000,
        &Some(meta.clone()),
    );

    let last_event = env.events().all().last().unwrap();
    assert_eq!(
        last_event.1,
        (symbol_short!("Stream"), symbol_short!("Created")).into_val(&env)
    );
    let event_data: StreamCreated = last_event.2.into_val(&env);
    assert_eq!(event_data.metadata, Some(meta));
}

/// Multiple key-value pairs survive the round-trip through storage.
#[test]
fn test_metadata_multiple_labels_round_trip() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&sender, &1000);

    let mut meta = Map::new(&env);
    meta.set(String::from_str(&env, "department"), String::from_str(&env, "engineering"));
    meta.set(String::from_str(&env, "project"), String::from_str(&env, "xlm-vesting"));
    meta.set(String::from_str(&env, "cost_center"), String::from_str(&env, "cc-42"));

    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000,
        &Some(meta.clone()),
    );

    let stream = client.get_stream(&stream_id);
    let stored = stream.metadata.unwrap();
    assert_eq!(
        stored.get(String::from_str(&env, "department")),
        Some(String::from_str(&env, "engineering"))
    );
    assert_eq!(
        stored.get(String::from_str(&env, "project")),
        Some(String::from_str(&env, "xlm-vesting"))
    );
    assert_eq!(
        stored.get(String::from_str(&env, "cost_center")),
        Some(String::from_str(&env, "cc-42"))
    );
}

// =============================================================================
// #119 — CLAWBACK TESTS
// =============================================================================

/// initialize stores the admin address.
#[test]
fn test_initialize_stores_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let compliance_admin = Address::generate(&env);
    client.initialize(&compliance_admin);
    // No panic → admin was stored successfully
}

/// Double-initialization panics with "already initialized".
#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_cannot_be_called_twice() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let compliance_admin = Address::generate(&env);
    client.initialize(&compliance_admin);
    client.initialize(&compliance_admin);
}

/// Admin can claw back up to the unclaimed vested amount.
#[test]
fn test_clawback_transfers_to_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let compliance_admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&sender, &1000);

    client.initialize(&compliance_admin);
    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );

    // At t=500, vested = 500, claimed = 0 → max clawback = 500
    env.ledger().with_mut(|l| l.timestamp = 500);
    let clawed = client.clawback(&stream_id, &300, &compliance_admin);

    assert_eq!(clawed, 300);
    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&compliance_admin), 300);
}

/// Clawback caps at unclaimed vested even when amount requested is larger.
#[test]
fn test_clawback_caps_at_unclaimed_vested() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let compliance_admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&sender, &1000);

    client.initialize(&compliance_admin);
    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );

    // At t=400, vested = 400 → requesting 1000 should be capped to 400
    env.ledger().with_mut(|l| l.timestamp = 400);
    let clawed = client.clawback(&stream_id, &1000, &compliance_admin);
    assert_eq!(clawed, 400);
}

/// After a clawback, recipient can only claim the remaining vested amount.
#[test]
fn test_clawback_reduces_recipient_claimable() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let compliance_admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&sender, &1000);

    client.initialize(&compliance_admin);
    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );

    // At t=500, vested = 500; admin claws back 200
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.clawback(&stream_id, &200, &compliance_admin);

    // Recipient should now only be able to claim 500 - 200 = 300
    assert_eq!(client.claimable(&stream_id, &500), 300);
    client.claim(&stream_id, &recipient, &300);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&recipient), 300);
    assert_eq!(token_client.balance(&compliance_admin), 200);
}

/// Non-admin callers panic with "unauthorized".
#[test]
#[should_panic(expected = "unauthorized")]
fn test_clawback_non_admin_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let compliance_admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&sender, &1000);

    client.initialize(&compliance_admin);
    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );

    env.ledger().with_mut(|l| l.timestamp = 500);
    // attacker != compliance_admin → should panic
    client.clawback(&stream_id, &100, &attacker);
}

/// Calling clawback before initialize panics with "contract not initialized".
#[test]
#[should_panic(expected = "contract not initialized")]
fn test_clawback_before_initialize_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let someone = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&sender, &1000);

    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.clawback(&stream_id, &100, &someone);
}

/// ClawbackExecuted event is emitted with correct fields.
#[test]
fn test_clawback_emits_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let token_admin = Address::generate(&env);
    let compliance_admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&sender, &1000);

    client.initialize(&compliance_admin);
    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.clawback(&stream_id, &250, &compliance_admin);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, contract_id);
    assert_eq!(
        last_event.1,
        (symbol_short!("Stream"), symbol_short!("Clawback")).into_val(&env)
    );
    let event_data: ClawbackExecuted = last_event.2.into_val(&env);
    assert_eq!(event_data.stream_id, stream_id);
    assert_eq!(event_data.amount, 250);
    assert_eq!(event_data.recipient, compliance_admin);
}

/// Token conservation: recipient claims + admin clawback = total vested at clawback time.
#[test]
fn test_clawback_token_conservation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StellarStreamContract);
    let client = StellarStreamContractClient::new(&env, &contract_id);

    let token_admin_addr = Address::generate(&env);
    let compliance_admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = create_token(&env, &token_admin_addr);
    let token_mint = token::StellarAssetClient::new(&env, &token);
    token_mint.mint(&sender, &1000);

    client.initialize(&compliance_admin);
    let stream_id = client.create_stream(
        &sender, &recipient, &token, &1000, &0, &1000, &None,
    );

    // Recipient claims 200 at t=400
    env.ledger().with_mut(|l| l.timestamp = 400);
    client.claim(&stream_id, &recipient, &200);

    // Admin claws back 100 at t=600 (vested=600, claimed=200, unclaimed=400)
    env.ledger().with_mut(|l| l.timestamp = 600);
    client.clawback(&stream_id, &100, &compliance_admin);

    // Remaining claimable for recipient = 600 - 200 - 100 = 300
    assert_eq!(client.claimable(&stream_id, &600), 300);
    client.claim(&stream_id, &recipient, &300);

    let token_client = token::Client::new(&env, &token);
    // recipient: 200 + 300 = 500, admin: 100, escrow holds 400 (unvested)
    assert_eq!(token_client.balance(&recipient), 500);
    assert_eq!(token_client.balance(&compliance_admin), 100);
}
