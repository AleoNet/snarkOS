mod common;

// Simulate a small network producing and agreeing on a single
// new block.
#[tokio::test]
#[ignore]
async fn process_one_block() {
    // Initiate the logger.
    let _ = tracing_subscriber::fmt().try_init();

    // The number of participants.
    const N: usize = 5;

    // Will be used to send messages to the consensus manager.
    let msg_senders = common::create_test_managers(N);

    // TODO: kick off the process

    // how to verify the results? (probably check the ledgers)
}

// This test verifies that a straightforward process works.
// Only the necessary messages are received by a single manager.
#[tokio::test]
#[ignore]
async fn accept_one_hardcoded_block() {
    // Initiate the logger.
    let _ = tracing_subscriber::fmt().try_init();

    // Will be used to send messages to the consensus manager.
    let msg_sender = common::create_test_managers(1).pop().unwrap();

    // TODO: provide the manager with a list of messages leading to a new block being accepted

    // how to verify the results? (probably check the ledger)
}
