# Latest Block Transactions
Returns the latest block transactions from the canonical chain.

### Arguments

None

### Response

| Parameter       |  Type  |                         Description                         |
|:---------------:|:------:|:-----------------------------------------------------------:|
| `transactions`  | array | The transactions of the latest block of the canonical chain. |

#### Transaction

|      Parameter      |  Type  | Description |
|:-------------------:|:------:|:-----------:|
| `events`            | array  | The events emitted from the transaction |
| `inner_circuit_id`  | string | The ID of the inner circuit used to execute each transition. |
| `ledger_root`       | string | The ledger root used to prove inclusion of ledger-consumed records. |
| `transaction_id`    | string | The ID of the transaction. |
| `transitions`       | array  | The state transitions. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblocktransactions", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
  "jsonrpc":"2.0",
  "result":{
    "transactions":[
      {
        "events":[

        ],
        "inner_circuit_id":"ic14z3rtzc25jgjxs6tzat3wtsrf5gees5zel8tggsslzrfyzhxw4xsgdfe4qk6997427zsfl9tqqesq5fzw5q",
        "ledger_root":"al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
        "transaction_id":"at1vkepkvxw29fnhh7g4m2q8r3tjj8cr8sk69p3w3e5df89tkkfzyps3qv3lt",
        "transitions":[
          {
            "ciphertext_ids":[
              "ar1k2rwg8tlg79zmgwzesyf4r3l2vyx27mtlrnjqanugx72l0jpfqxqt7sf3u",
              "ar175fezhx60h7e0c5nfv99ke3fhhwms7wrreq6t4pv538mnknksgfq04333e"
            ],
            "ciphertexts":[
              "a379350c60e4c22bd8ff75808b90cf4fa8f0ef887386f60ce1af948e0069ed06f46382f7d8a2a7f1ba1435492b5f327bb7e91d16cc4939ee6c69bc56f6264409a3a91dea4599e7ec489816cfd7e966242deecb669b13a7175ca6d49b9e6cbe0a9accda04da9532850f35ef3ae6ab13a726435fb00154774e9daea6c07b627110c9edf3b477cb2850e2734a1b0fefd1a99406e8620a0f4b1549b68f9cbb112d06161487d93570f0be62e383fdc0bc80f4c53a729f44bfc2c2f1a193e0d5cfd1071d04219124da47a6b172cd55c819b6fa5450561a2aa32b489ceecadedb02280a179dc270acc9fcdfc2d3c91765c3a01011824b878e87f9206adcd683e7b5a008e26f7a24ecf8d0ee4031fc76b5cc8025324755d2becf2279ddd1ae1b7f4ffa0350b5686dbe317b77a46e5609408fc2e4497fb003e1efd306d4a5445c742f3b04",
              "b2d1ed016804bdda77e515ec41eca8aaae48ae93eb943a67dac645b174cdfe0738945884b114001c58921450eff0ae0aafd294ffc4d383812b2e9d58f9ea7d0a66d30dc5951df95b77bf565d293d65e2aeeaa62f5e39440d97bbb996acd34106c0b64e53adee41e48b195f143ad1089f9314554260a1786f3c588e8093a4d1090b23fa322183893f57497fc661eaefa1a3f6565d755a7bae2465e8433240c90e16b15de9decccc4e67550788866b5730e454bfd45b51ba453e3a07402bf2070f732d63878c6671f6b109d9af312e6ce259bac24f48157f7d3c5c6af600204d079b08ace49b32afd44e58c5ec6d6d500fc862b5862b9daf8374a438b364bcef0c198a0657f0935551e8b115f3bcf154b9c0470164eef0812a56ba2d97fdb2df012de08f8e59be6cf253323915164fb34005718ef61fdff647d1b9ad096d104e10"
            ],
            "commitments":[
              "cm162h6gpq9cdtjk9usfwh7hkjw7zm9uxm9f7ey93zsu6alyev3tcps90k0an",
              "cm1qs29nvxnkn8utcgmhtpmyzsxwxdfck4884ujwss3ff2zx60alupsdy6t39"
            ],
            "proof":"ozkp1t87t3wr8tygarh58sqy5s3xuzlgw8syxe6902ke3zznmfh0us2ywltgaw88yhx39qxpenfugvh3r2nldnnp88zj2avkpnrrvckark2encwc7wt9j6e7rv062rzt3r2mxrq5zmvlmtqpdm3gwlkh0f0waqz3ah7zm7p4jpmw77lw75vwey3y6uu4uvn7rrf0su06zjh3m6jhj93020djnscmt8y3rdnvrwpa25yl4a92njkjt8pylr7l0rn900rjqy5a72qhyk3vyruezxhrpt9wsn75h3j68juz027jzt5xkp7vuy2q36f0ldqhsaylmrmdj65tvsv3dhtxqkl97429wqy6sn7ay2kqk67tm4v2wp53p47pv6lxfzj0us0k0q9e393s73seane3xekw6c673plq6kqve95dxvlvc4mcal6h76w60wm2n5d062awlrl8e7g7tmv3qzqg84vp38",
            "serial_numbers":[
              "sn12e9v23npctlrtpdmv02gxawj9kjfjs3u98l7lqles9e2y4kwfqgqkmjp5w",
              "sn1mg7jlqsyk0fetjy376g6tjf5pylyqp0mhc0h5nl90dvaqahsuuxstly6mt"
            ],
            "transition_id":"as1ltse2tzr5wysqv8xtepdw4zq5mvz4nkgfxmcp7vs694acym80yzsel6h2j",
            "value_balance":-150000000
          }
        ]
      }
    ]
  },
  "id":"1"
}
```