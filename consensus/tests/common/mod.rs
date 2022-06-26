use tokio::sync::mpsc;
use tracing::*;

use snarkos_consensus::reference::{
    message::{Message, TestMessage},
    validator::Validator,
};

// Spawns the desired number of consensus managers within their dedicated tasks
// and returns a list of senders that can be used to send individual messages.
pub fn create_test_managers(num: usize) -> Vec<mpsc::Sender<Message>> {
    // This channel simulates network communication.
    let (common_msg_sender, mut common_msg_receiver) = mpsc::channel(64);

    // This collection holds senders used to send messages TO the consensus managers.
    let mut individual_msg_senders = Vec::with_capacity(num);

    // Create consensus managers and spawn their dedicated tasks.
    for i in 0..num {
        // This is a channel dedicated to sending messages TO the consensus manager.
        let (msg_sender, mut msg_receiver) = mpsc::channel(16);

        // The sender passed here is for sending messages FROM the consensus manager.
        let mut manager = Validator::new(common_msg_sender.clone());

        // Spawn a dedicated consensus manager task.
        let _manager_task = tokio::spawn(async move {
            debug!("Spawned a task for consensus manager {}", i);

            // Handle consensus messages.
            while let Some(msg) = msg_receiver.recv().await {
                debug!("Consensus manager {} received a message: {:?}", i, msg);
                manager.start_event_processing(msg);
            }
        });

        // TODO: create a map from manager indices to their signatures

        individual_msg_senders.push(msg_sender);
    }

    // Spawn a task simulating a network and passing around messages.
    let individual_senders = individual_msg_senders.clone();
    let _network_simulating_task = tokio::spawn(async move {
        debug!("Spawned a task simulating a network");

        // Redirect messages, simulating a network.
        while let Some(msg) = common_msg_receiver.recv().await {
            if let Some(target) = msg.target {
                // Send a direct message to the specified target.
                individual_senders[target].send(msg.message).await.unwrap();
            } else {
                // Send a message to all the members of the network.
                // TODO: don't send to the author (based on the signature)
                for sender in &individual_senders {
                    sender.send(msg.message.clone()).await.unwrap();
                }
            }
        }
    });

    individual_msg_senders
}
