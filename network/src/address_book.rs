use chrono::{DateTime, Utc};
use std::{collections::HashMap, net::SocketAddr};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddressBook {
    /// Hashmap of addresses
    pub addresses: HashMap<SocketAddr, DateTime<Utc>>,
    // TODO @raychu Add structures to store addresses like this:
    //    by_addr: HashMap<SocketAddr, (DateTime<Utc>, ServicesProvided)>,
    //    by_time: BTreeSet<MetaAddr>,
}

impl AddressBook {
    pub fn new() -> Self {
        Self {
            addresses: HashMap::default(),
        }
    }

    /// Update the stored
    pub fn update(&mut self, addr: SocketAddr, date: DateTime<Utc>) -> bool {
        match self.addresses.get(&addr) {
            Some(stored_date) => {
                if stored_date > &date {
                    false
                } else {
                    self.addresses.insert(addr, date);
                    true
                }
            }
            None => {
                self.addresses.insert(addr, date);
                true
            }
        }
    }

    /// Remove an item
    pub fn remove(&mut self, addr: &SocketAddr) -> Option<DateTime<Utc>> {
        self.addresses.remove(addr)
    }
}
