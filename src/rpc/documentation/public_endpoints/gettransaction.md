# Get Transaction
Returns a transaction with metadata given the transaction ID.

### Arguments

|     Parameter    |  Type  | Required |                   Description                   |
|:----------------:|:------:|:--------:|:-----------------------------------------------:|
| `transaction_id` | string |    Yes   | The transaction id of the requested transaction |

### Response

|      Parameter      |  Type  | Description |
|:-------------------:|:------:|:-----------:|
| `metadata`          | object | The metadata of the requested transaction |
| `transaction`       | object | The transaction object |

#### Transaction Metadata

|      Parameter      |  Type  | Description |
|:-------------------:|:------:|:-----------:|
| `block_hash`        | string | The block hash of the block the transaction is included in. |
| `block_height`      | number | The block height of the block the transaction is included in. |
| `block_timestamp`   | number | The block timestamp of the block the transaction is included in. |
| `transaction_index` | number | The index of the transaction in the block. |

#### Transaction

|      Parameter      |  Type  | Description |
|:-------------------:|:------:|:-----------:|
| `events`            | array  | The events emitted from the transaction |
| `inner_circuit_id`  | string | The ID of the inner circuit used to execute each transition. |
| `ledger_root`       | string | The ledger root used to prove inclusion of ledger-consumed records. |
| `transaction_id`    | string | The ID of this transaction. |
| `transitions`       | array  | The state transitions. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "gettransaction", "params": ["at1mka6m3kfsgt5dpnfurk2ydjefqjzng4aawj7lkpc32pjkg86hyysrke9nf"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response 
```json
{
   "jsonrpc":"2.0",
   "result":{
      "metadata":{
         "block_hash":"ab18z5tqgels9vrqmq6rygwxwfhj9rn8273wdjkh0fwykyz2l7l0srqa5jl8g",
         "block_height":6471,
         "block_timestamp":1636492571,
         "transaction_index":0
      },
      "transaction":{
         "events":[
            
         ],
         "inner_circuit_id":"ic14z3rtzc25jgjxs6tzat3wtsrf5gees5zel8tggsslzrfyzhxw4xsgdfe4qk6997427zsfl9tqqesq5fzw5q",
         "ledger_root":"al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
         "transaction_id":"at1mka6m3kfsgt5dpnfurk2ydjefqjzng4aawj7lkpc32pjkg86hyysrke9nf",
         "transitions":[
            {
               "ciphertext_ids":[
                  "ar102yu5z68da204tplj4zs326jhnk02za7wceg6pnnv5mu9t8wpvqq3yvsks",
                  "ar18q4twnjfw93yh8k440ypx9mkyvhmcwnm4s627gw6cptjyh85jqzqxhfl53"
               ],
               "ciphertexts":[
                  "6ae72cc574bbf433dcd715936cc7ca0b5ad64f34e5e7d9065de862293980410577adf0004c18898534ab5abb2e037ceeafadd3cfc9b1cfad231b5488b12dfc11223b691edd66acd238ab08b9c93df4ebbb7fdacd7e99c26718676176992f83049142a55d5eb9bdbdd219d421bdabc3efa87c6f1d249ed79d69569e4bc0054202adea2b4bea16548e12536e45629345fa0f37c0ce9f629d16af3c87c08776da0efc7defc571fe32d33cb9071f5a7b32da42133211cd3de62bd5083956ae9f9609296aedc1e64282a9f2787f543a3dfe97486d79e508332fda2e6b050b0845e702d12d0af378dd70cb275c974f4e46107fa8503c521d65df17e523b1828fe40500c78c1f38de4179df61b3fba2e43f42cddc5d37d019c440819ca3a1e32efa960a004086347a10973bbe8faf621d1b57c33fce66463552b7fd794523a8ea77b104",
                  "b2016b1196291b6898296d86f1b351314b5440274f0f231ceb0bd6e2355e4a0d71f711cc8a3861d0d6e2f22913645225be41a60d332ed315b080061e498f6103d8a0034bbad2473f5819202e895d97282e9e163e6cb9a9767878d0978fd42505aede5841b9931e80171b1fe9aa13b6e2d2c8efaf619d79bf121b2bbdef132e0743e8e44ab89ba3d9a9f8a5cacadec5de5f7529d63b6f379201468c7cb6c0c90da4a6d5982f7ad4ea98883a20b7b4beb87b3675b3819d96dc7b3fb747e35df30fa8ba03907c80b649931a445f07bf89daeccf9ca2569f8cc93a280ca5018007030f109b30bdce29aa322c925a973ff15cbb17507ca607dce69eb742ca9e956a0d2294656daad7363ad148eb03c35457094c9eea8a7c87de73bd2bb4ee6e9649031544a52e382bbd26fbbe7f03e8821a4e39bf4491cb64de75e1c33c87bddfca0f"
               ],
               "commitments":[
                  "cm1guhe607n4vthtxcfnhmheref4mzu090lkrfmzw5r3s74qg5vvvzqc82y8p",
                  "cm17g8s6aca957v5e86gy2nvcjtau82j5zs6rylmzfkfqhg4hzs8cps6e6zhg"
               ],
               "proof":"ozkp140k7v0jrpwslj5j25y3tyhdlp0hzkw4jwcmdr09xzn6l6md5zr72w7vqd9fvkg3jv9ddfphkys2zzwhavc678rj39qlg2jafaars6txwv64zwzaj9l7mvjqd3kwujunl0jge8k7ah9um87gqlxxlwdcmsr3ehqqtnnltqt3tx0kaes2f8yhs4lwl8q9772334znemuyje46s5f5fkurlhxh0e4rzj9a38579nmtnfrd2xunr8lhzvygnnhxsjl6d285acv3seq46fe3ncnklff57qhht64cn80ucns3jt3mp0lu0pwqsqydkdpkyplr9pwlzuhf3car59ra6zlez5se8fgvf0l8qm68d22f28lhj2sdppfnty5wsrsssm8gg06q43l2l82y3gpm3ttzakg4kvd8yhl5czv9te5cxqn9u5jstmq6q749xkw62va724qwkrnpsc5zgqqghwmlxe",
               "serial_numbers":[
                  "sn1429p78ykjz8v9haw69nu7ky22hjzcqjqj0wph9ptlrqzm5r5jcpq5h9fpk",
                  "sn1z7wdn0spy4q6vgjs9dqgu6jt7njasufzgfalhvwrnkjjmnrdzvyse5kdjk"
               ],
               "transition_id":"as1v7dvwcwy8430kvt74chprep904mvw4wmy3svll4tlt7vqpgutspsf79ek0",
               "value_balance":-150000000
            }
         ]
      }
   },
   "id":"1"
}
```