use crate::{option, subcommand, types::*, CLIError, CLI};

use clap::{ArgMatches, Values};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Map, Value};
use std::{collections::HashMap, error::Error};

/// Represents options for a Bitcoin wallet
#[derive(Clone, Debug, Serialize)]
pub struct ConsoleConfig {
    // Options
    pub ip: String,
    pub port: u16,
    pub method: String,
    pub params: Vec<Value>,

    //Subcommand
    subcommand: Option<String>,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            // Options
            ip: "0.0.0.0".into(),
            port: 3030,
            method: "getblockcount".into(),
            params: vec![],
            subcommand: None,
        }
    }
}

impl ConsoleConfig {
    fn parse(&mut self, arguments: &ArgMatches, options: &[&str]) {
        options.iter().for_each(|option| match *option {
            // Options
            "ip" => self.ip(arguments.value_of(option)),
            "port" => self.port(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "getbalance" => self.get_balance(arguments.value_of(option)),
            "getblock" => self.get_block(arguments.value_of(option)),
            "getblockcount" => self.get_block_count(arguments.is_present(option)),
            "getbestblockhash" => self.get_best_block_hash(arguments.is_present(option)),
            "listunspent" => self.list_unspent(arguments.value_of(option)),
            "getrawtransaction" => self.get_raw_transaction(arguments.value_of(option)),
            "createrawtransaction" => self.create_raw_transaction(arguments.values_of(option)),
            "decoderawtransaction" => self.decode_raw_transaction(arguments.value_of(option)),
            "signrawtransaction" => self.sign_raw_transaction(arguments.values_of(option)),
            "sendrawtransaction" => self.send_raw_transaction(arguments.value_of(option)),
            "getconnectioncount" => self.get_connection_count(arguments.is_present(option)),
            "getpeerinfo" => self.get_peer_info(arguments.is_present(option)),
            "getblocktemplate" => self.get_block_template(arguments.is_present(option)),
            _ => (),
        });
    }

    fn ip(&mut self, argument: Option<&str>) {
        if let Some(ip) = argument {
            self.ip = ip.to_string();
        }
    }

    fn port(&mut self, argument: Option<u16>) {
        if let Some(port) = argument {
            self.port = port;
        }
    }

    fn get_balance(&mut self, argument: Option<&str>) {
        if let Some(address) = argument {
            self.method = "getbalance".to_string();
            self.params = vec![Value::String(address.to_string())];
        }
    }

    fn get_block(&mut self, argument: Option<&str>) {
        if let Some(block_hash) = argument {
            self.method = "getblock".to_string();
            self.params = vec![Value::String(block_hash.to_string())];
        }
    }

    fn get_block_count(&mut self, argument: bool) {
        if argument {
            self.method = "getblockcount".to_string();
            self.params = vec![];
        }
    }

    fn get_best_block_hash(&mut self, argument: bool) {
        if argument {
            self.method = "getbestblockhash".to_string();
            self.params = vec![];
        }
    }

    fn list_unspent(&mut self, argument: Option<&str>) {
        if let Some(address) = argument {
            self.method = "listunspent".to_string();
            self.params = vec![Value::String(address.to_string())];
        }
    }

    fn get_raw_transaction(&mut self, argument: Option<&str>) {
        if let Some(transaction_id) = argument {
            self.method = "getrawtransaction".to_string();
            self.params = vec![Value::String(transaction_id.to_string())];
        }
    }

    fn create_raw_transaction(&mut self, argument: Option<Values>) {
        if let Some(transaction_parameters) = argument {
            self.method = "createrawtransaction".to_string();

            let params: Vec<&str> = transaction_parameters.collect();

            let raw_inputs: Vec<RPCTransactionOutpoint> = from_str(params[0]).unwrap();
            let inputs = Value::Array(raw_inputs.iter().map(|input| json![input]).collect::<Vec<Value>>());

            let raw_outputs: HashMap<String, Value> = from_str(params[1]).unwrap();
            let mut outputs = Map::new();
            for (address, value) in raw_outputs {
                outputs.insert(address, value);
            }

            self.params = vec![inputs, Value::Object(outputs)];
        }
    }

    fn decode_raw_transaction(&mut self, argument: Option<&str>) {
        if let Some(transaction_bytes) = argument {
            self.method = "decoderawtransaction".to_string();
            self.params = vec![Value::String(transaction_bytes.to_string())];
        }
    }

    fn sign_raw_transaction(&mut self, argument: Option<Values>) {
        if let Some(transaction_parameters) = argument {
            self.method = "signrawtransaction".to_string();
            self.params = vec![];
            let params: Vec<&str> = transaction_parameters.collect();

            let transaction_bytes = Value::String(params[0].to_string());

            let private_keys: Vec<&str> = from_str(params[1]).unwrap();
            let private_keys = Value::Array(private_keys.iter().map(|input| json![input]).collect::<Vec<Value>>());

            self.params = vec![transaction_bytes, private_keys];
        }
    }

    fn send_raw_transaction(&mut self, argument: Option<&str>) {
        if let Some(transaction_bytes) = argument {
            self.method = "sendrawtransaction".to_string();
            self.params = vec![Value::String(transaction_bytes.to_string())];
        }
    }

    fn get_connection_count(&mut self, argument: bool) {
        if argument {
            self.method = "getconnectioncount".to_string();
            self.params = vec![];
        }
    }

    fn get_peer_info(&mut self, argument: bool) {
        if argument {
            self.method = "getpeerinfo".to_string();
            self.params = vec![];
        }
    }

    fn get_block_template(&mut self, argument: bool) {
        if argument {
            self.method = "getblocktemplate".to_string();
            self.params = vec![];
        }
    }
}

pub struct ConsoleCli;

impl CLI for ConsoleCli {
    type Config = ConsoleConfig;

    const ABOUT: AboutType = "Make RPC calls to a fullnode (include -h for more options)";
    const FLAGS: &'static [FlagType] = &[];
    const NAME: NameType = "skeleton-console";
    const OPTIONS: &'static [OptionType] = &[
        option::IP,
        option::PORT,
        option::GET_BALANCE,
        option::GET_BLOCK,
        option::GET_BLOCK_COUNT,
        option::GET_BEST_BLOCK_HASH,
        option::LIST_UNSPENT,
        option::GET_RAW_TRANSACTION,
        option::CREATE_RAW_TRANSACTION,
        option::DECODE_RAW_TRANSACTION,
        option::SIGN_RAW_TRANSACTION,
        option::SEND_RAW_TRANSACTION,
        option::GET_CONNECTION_COUNT,
        option::GET_PEER_INFO,
        option::GET_BLOCK_TEMPLATE,
    ];
    const SUBCOMMANDS: &'static [SubCommandType] = &[subcommand::TEST_SUBCOMMAND];

    /// Handle all CLI arguments and flags for skeleton node
    fn parse(arguments: &ArgMatches) -> Result<Self::Config, CLIError> {
        let mut config = ConsoleConfig::default();
        config.parse(arguments, &[
            "ip",
            "port",
            "getbalance",
            "getblock",
            "getblockcount",
            "getbestblockhash",
            "listunspent",
            "getrawtransaction",
            "createrawtransaction",
            "decoderawtransaction",
            "signrawtransaction",
            "sendrawtransaction",
            "getconnectioncount",
            "getpeerinfo",
            "getblocktemplate",
        ]);

        // TODO: remove this for release
        match arguments.subcommand() {
            ("test", Some(arguments)) => {
                config.subcommand = Some("test".into());
                config.parse(arguments, &[
                    "ip",
                    "port",
                    "getbalance",
                    "getblock",
                    "getblockcount",
                    "getbestblockhash",
                    "listunspent",
                    "getrawtransaction",
                    "createrawtransaction",
                    "decoderawtransaction",
                    "signrawtransaction",
                    "sendrawtransaction",
                    "getconnectioncount",
                    "getpeerinfo",
                    "getblocktemplate",
                ]);
            }
            _ => {}
        }
        Ok(config)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionOutpoint {
    /// Previous transaction id
    pub txid: String,
    /// Previous transaction output index
    pub vout: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionInputs(pub Vec<RPCTransactionOutpoint>);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionOutputs(pub HashMap<String, u64>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Vec<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    #[serde(rename(deserialize = "jsonrpc"))]
    pub json_rpc: String,
    pub result: Option<Value>,
    pub error: Option<RpcResponseError>,
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcResponseError {
    pub code: i64,
    pub message: String,
}

impl ConsoleCli {
    pub async fn make_request(arguments: ConsoleConfig) -> Result<String, Box<dyn Error>> {
        let url = format!("http://{}:{}", arguments.ip, arguments.port);

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: arguments.method,
            params: arguments.params,
        };

        let client = Client::new();
        let res = client.post(&url).json(&request).send().await?.text().await?;

        let response: RpcResponse = from_str(&res)?;

        match (response.result, response.error) {
            (Some(val), None) => Ok(val.to_string()),
            (None, Some(error)) => Ok(error.message),
            (_, _) => unreachable!(),
        }
    }
}
