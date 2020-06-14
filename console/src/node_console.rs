use crate::{option, types::*, CLIError, CLI};

use clap::ArgMatches;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, Value};
use std::error::Error;

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
            "getblock" => self.get_block(arguments.value_of(option)),
            "getblockcount" => self.get_block_count(arguments.is_present(option)),
            "getbestblockhash" => self.get_best_block_hash(arguments.is_present(option)),
            "getblockhash" => self.get_block_hash(arguments.value_of(option)),
            "getrawtransaction" => self.get_raw_transaction(arguments.value_of(option)),
            "gettransactioninfo" => self.get_transaction_info(arguments.value_of(option)),
            "decoderawtransaction" => self.decode_raw_transaction(arguments.value_of(option)),
            "sendrawtransaction" => self.send_raw_transaction(arguments.value_of(option)),
            "decoderecord" => self.decode_record(arguments.value_of(option)),
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

    fn get_block_hash(&mut self, argument: Option<&str>) {
        if let Some(block_hash) = argument {
            self.method = "getblockhash".to_string();
            self.params = vec![Value::String(block_hash.to_string())];
        }
    }

    fn get_raw_transaction(&mut self, argument: Option<&str>) {
        if let Some(transaction_id) = argument {
            self.method = "getrawtransaction".to_string();
            self.params = vec![Value::String(transaction_id.to_string())];
        }
    }

    fn get_transaction_info(&mut self, argument: Option<&str>) {
        if let Some(transaction_id) = argument {
            self.method = "gettransactioninfo".to_string();
            self.params = vec![Value::String(transaction_id.to_string())];
        }
    }

    fn decode_raw_transaction(&mut self, argument: Option<&str>) {
        if let Some(transaction_bytes) = argument {
            self.method = "decoderawtransaction".to_string();
            self.params = vec![Value::String(transaction_bytes.to_string())];
        }
    }

    fn send_raw_transaction(&mut self, argument: Option<&str>) {
        if let Some(transaction_bytes) = argument {
            self.method = "sendrawtransaction".to_string();
            self.params = vec![Value::String(transaction_bytes.to_string())];
        }
    }

    fn decode_record(&mut self, argument: Option<&str>) {
        if let Some(record_bytes) = argument {
            self.method = "decoderecord".to_string();
            self.params = vec![Value::String(record_bytes.to_string())];
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
        option::GET_BLOCK,
        option::GET_BLOCK_COUNT,
        option::GET_BEST_BLOCK_HASH,
        option::GET_BLOCK_HASH,
        option::GET_RAW_TRANSACTION,
        option::GET_TRANSACTION_INFO,
        option::DECODE_RAW_TRANSACTION,
        option::SEND_RAW_TRANSACTION,
        option::DECODE_RECORD,
        option::GET_CONNECTION_COUNT,
        option::GET_PEER_INFO,
        option::GET_BLOCK_TEMPLATE,
    ];
    const SUBCOMMANDS: &'static [SubCommandType] = &[];

    /// Handle all CLI arguments and flags for skeleton node
    fn parse(arguments: &ArgMatches) -> Result<Self::Config, CLIError> {
        let mut config = ConsoleConfig::default();
        config.parse(arguments, &[
            "ip",
            "port",
            "getblock",
            "getblockcount",
            "getbestblockhash",
            "getblockhash",
            "getrawtransaction",
            "gettransactioninfo",
            "decoderawtransaction",
            "sendrawtransaction",
            "decoderecord",
            "getconnectioncount",
            "getpeerinfo",
            "getblocktemplate",
        ]);

        Ok(config)
    }
}

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
