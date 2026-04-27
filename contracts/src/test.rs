#![cfg(test)]
extern crate std;
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env, IntoVal, Vec, symbol_short,
};
use insta::assert_debug_snapshot as assert_snapshot;

fn create_token(env: &Env, admin: &Address) -> Address {
    let token_contract_id = env.register_stellar_asset_contract_v2(admin.clone());
    token_contract_id.address()
}

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
    client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000);
    assert_eq!(client.get_next_stream_id(), 1);
    client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000);
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
    client.create_stream(&sender, &recipient, &token, &0, &0, &1000);
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
    client.create_stream(&sender, &recipient, &token, &1000, &1000, &1000);
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000);
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
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &1000, &2000);
    assert_eq!(client.claimable(&stream_id, &2100), 1000);
}



// -----------------------------------------------------------------
// CANCEL BEFORE STREAM START
// -----------------------------------------------------------------

/// Cancel issued before the stream's start_time.
/// vested = 0, so the full total_amount must be returned to the sender.
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

    // Stream starts at t=500, current ledger time is 0 (before start)
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &500, &1500);

    // Sender balance is 0 after escrow; cancel at t=0 (before start_time=500)
    env.ledger().with_mut(|l| l.timestamp = 0);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    // Full amount must come back to sender
    assert_eq!(token_client.balance(&sender), 1000);
    // Recipient must have received nothing
    assert_eq!(token_client.balance(&recipient), 0);
}

/// After a pre-start cancel the recipient's claimable amount must be zero.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &500, &1500);

    env.ledger().with_mut(|l| l.timestamp = 0);
    client.cancel(&stream_id, &sender);

    // claimable at any future time must be 0 because end_time was truncated
    assert_eq!(client.claimable(&stream_id, &1500), 0);
    assert_eq!(client.claimable(&stream_id, &9999), 0);
}

/// Attempting to claim any amount after a pre start cancel must panic.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &500, &1500);

    env.ledger().with_mut(|l| l.timestamp = 0);
    client.cancel(&stream_id, &sender);

    // Advance time well past original end — should still be unclaim­able
    env.ledger().with_mut(|l| l.timestamp = 2000);
    client.claim(&stream_id, &recipient, &1);
}

// -----------------------------------------------------------------
// CANCEL MID-STREAM — partial vesting refund math
// -----------------------------------------------------------------

/// Cancel at exactly the 25 % mark.
/// vested = 250, sender refund = 750, recipient can claim 250.
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

    // Stream: t=0..1000, total=1000 → 1 token er second
    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    env.ledger().with_mut(|l| l.timestamp = 250); // 25 % through
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    // Sender must receive the unvested 75 %
    assert_eq!(token_client.balance(&sender), 750);

    // Recipient can now claim exactly the vested 25 %
    client.claim(&stream_id, &recipient, &250);
    assert_eq!(token_client.balance(&recipient), 250);
}

/// Cancel at exactly the 50 % mark with no prior claims.
/// vested = 500, sender refund = 500.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 500);

    // Recipient claims vested portion
    client.claim(&stream_id, &recipient, &500);
    assert_eq!(token_client.balance(&recipient), 500);

    // Total accounted for = 1000, nothing lost or double-counted
    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient),
        1000
    );
}

/// Cancel at 75 % with a non-round total to verify integer arithmetic is exact.
/// total=1200, duration=1200s, cancel at t=900 (75 %)
/// vested = 900, sender refund = 300.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1200, &0, &1200);

    env.ledger().with_mut(|l| l.timestamp = 900); // 75 %
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 300);
    assert_eq!(client.claimable(&stream_id, &900), 900);
}

/// Recipient claims some tokens mid-stream, then sender cancels.
/// Refund = total - vested (NOT total - claimed); the contract holds
/// vested - claimed for the recipient and returns total - vested to sender.
/// total=1000, cancel at t=600 → vested=600, already_claimed=200
/// sender_refund = 400, remaining_claimable_for_recipient = 400
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    // Recipient claims 200 at t=400 (vested=400 at that point)
    env.ledger().with_mut(|l| l.timestamp = 400);
    client.claim(&stream_id, &recipient, &200);

    // Sender cancels at t=600 (vested=600)
    env.ledger().with_mut(|l| l.timestamp = 600);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    // Sender gets back 1000 - 600 = 400
    assert_eq!(token_client.balance(&sender), 400);

    // Recipient already has 200; can claim remaining vested - claimed = 400
    assert_eq!(client.claimable(&stream_id, &600), 400);
    client.claim(&stream_id, &recipient, &400);
    assert_eq!(token_client.balance(&recipient), 600); // 200 + 400

    // Full conservation: 400 (sender) + 600 (recipient) = 1000
    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient),
        1000
    );
}

/// After a mid-stream cancel the recipient cannot claim more than what
/// was vested at cancel time, even if ledger time advances further.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    env.ledger().with_mut(|l| l.timestamp = 500);
    client.cancel(&stream_id, &sender);

    // Try to claim 501  one more than vested at cancel time
    env.ledger().with_mut(|l| l.timestamp = 2000); // far future
    client.claim(&stream_id, &recipient, &501);
}

// -----------------------------------------------------------------
// CANCEL AFTER FULL VESTING
// -----------------------------------------------------------------

/// Cancel exactly at end_time: all tokens are vested, sender gets 0 refund.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    env.ledger().with_mut(|l| l.timestamp = 1000); // exactly at end
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    // Sender transferred 1000 into escrow and should get 0 back
    assert_eq!(token_client.balance(&sender), 0);
}

/// Cancel well after end_time: full amount is vested, sender gets 0 refund.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    env.ledger().with_mut(|l| l.timestamp = 9999); // long after end
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 0);
}

/// After a post-full-vesting cancel the recipient can still claim the
/// entire total_amount.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    env.ledger().with_mut(|l| l.timestamp = 9999);
    client.cancel(&stream_id, &sender);

    // Recipient should be able to claim the full amount
    assert_eq!(client.claimable(&stream_id, &9999), 1000);
    client.claim(&stream_id, &recipient, &1000);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&recipient), 1000);
}

/// Cancel after full vesting when recipient has already claimed part:
/// recipient claims the remainder, sender still gets 0.
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

    let stream_id = client.create_stream(&sender, &recipient, &token, &1000, &0, &1000);

    // recipient claims 300 before stream ends
    env.ledger().with_mut(|l| l.timestamp = 500);
    client.claim(&stream_id, &recipient, &300);

    // Sender cancels after full vesting
    env.ledger().with_mut(|l| l.timestamp = 1500);
    client.cancel(&stream_id, &sender);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&sender), 0);

    // Recipient can claim the remaining 700
    assert_eq!(client.claimable(&stream_id, &1500), 700);
    client.claim(&stream_id, &recipient, &700);
    assert_eq!(token_client.balance(&recipient), 1000);
}

// -----------------------------------------------------------------
// CONSERVATION INVARIANT
// -----------------------------------------------------------------

/// For any cancel timing, the sum of tokens held by sender and recipient
/// after all claims must equal the original total_amount. This test
/// exercises three timings in one scenario using three independent streams.
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
    // Mint enough for three streams of 1200 each
    token_admin.mint(&sender, &3600);

    let token_client = token::Client::new(&env, &token);

    // --- Stream A: cancel before start ---
    let a = client.create_stream(&sender, &recipient, &token, &1200, &500, &1700);
    env.ledger().with_mut(|l| l.timestamp = 100); // before start_time=500
    let sender_before_a = token_client.balance(&sender);
    let recipient_before_a = token_client.balance(&recipient);
    client.cancel(&a, &sender);
    // Full refund to sender; recipient gets nothing
    assert_eq!(
        token_client.balance(&sender) + token_client.balance(&recipient)
            - sender_before_a
            - recipient_before_a,
        1200
    );

    // --- Stream B: cancel at 50 % ---
    env.ledger().with_mut(|l| l.timestamp = 0);
    let b = client.create_stream(&sender, &recipient, &token, &1200, &0, &1200);
    env.ledger().with_mut(|l| l.timestamp = 600); // 50 %
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

    // --- Stream C: cancel after full vesting ---
    env.ledger().with_mut(|l| l.timestamp = 0);
    let c = client.create_stream(&sender, &recipient, &token, &1200, &0, &1200);
    env.ledger().with_mut(|l| l.timestamp = 9999); // after end
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