# Get Memory Pool
Returns the transactions in the node's current transaction memory pool.

### Arguments

None

### Response

|     Parameter         |  Type  |                Description               |
|:---------------------:|:------:|:----------------------------------------:|
| `result`              |  array | The array of transactions                |


#### Transaction

|      Parameter     |  Type  |                             Description                             |
|:------------------:|:------:|:-------------------------------------------------------------------:|
| `events`           | array  | The events emitted from the transaction                             |
| `inner_circuit_id` | string | The ID of the inner circuit used to execute each transition.        |
| `ledger_root`      | string | The ledger root used to prove inclusion of ledger-consumed records. |
| `transaction_id`   | string | The ID of this transaction.                                         |
| `transitions`      | array  | The state transitions.                                              |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getmemorypool", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
   "jsonrpc":"2.0",
   "result":[
     {
       "events":[

       ],
       "inner_circuit_id":"ic14z3rtzc25jgjxs6tzat3wtsrf5gees5zel8tggsslzrfyzhxw4xsgdfe4qk6997427zsfl9tqqesq5fzw5q",
       "ledger_root":"al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
       "transaction_id":"at1pazplqjlhvyvex64xrykr4egpt77z05n74u5vlnkyv05r3ctgyxs0cgj6w",
       "transitions":[
         {
           "ciphertext_ids":[
             "ar18gr9hxzr40ve9238eddus8vq7ka8a07wk63666hmdqk7ess5mqqsh5xazm",
             "ar1qd0aga9fs724frhws2hp6dsfr5k7fmmtdllm7dupkp9ez85xlcpqnh4kcs"
           ],
           "ciphertexts":[
             "580a3b830d17818b866c8f4f7bc4f518dcd775adce1274e902d07edc49e3e9005418da88939bac4437a271cb34cf792e3bad3385fe05ec086b5e5c485b291209a7ced29b1f6f169dd13504a71ce57c3348fe8778c7a102e26791fb298a7a0a0edfae8e065599cd3869d08a91ffa06fb6e41f68033d9b8d9269fa6f948f167f07c7d57ab6f3198c3d568a60525cb4959df7655dc8de9751d3cd8ec6b5aa834c0b0b081c6734fb62670b80cc885210392e6f56cd2b4aec22c53257a3422fe474041540a950c102b00c36cb6e889a7cefe2822a003e9332f3536a18fd3bc1690e11094593d25543c3df7e2fc4a24754eafdf4e0002ed4673c1a84c49ddcc510870efef1b4f95f0cf7d1a2e0e754eaa188a22cefb4938e616506154135091874c10ed4abe8bf94915a782e545acc07663237c2c826fd2c516fd14a2f9d2c570afe0d",
             "3bba8e8ac95b43c3bc4be5792cf73f05458113641d7b15946e169188a7534f0ff6b7328b7549d042dfaeea13b81f8ea6eac72d68ea68681874a823fd08fa44043f074b48823758688ab1114ab0b5f2143ceaf1d8ef448ab21fd874a4e3ffb30919b8cddae76870d0e766fb43e847cb6f02fc18041e709695f9b03ffea5d7f206977a06b37c5a0f8b1a576b351233446bfaf6f8cc310bfcc1fa1138c310a3810d98f6cba62e95230e7966b59d3e383ab87a9f61760922b538e81004e6ab7efb02d811a7198af891db3eaa2d4e589643cf1d0f3c2b87c39d5d2cf163a59728b001d3e23485afe77843d2e3ab8554face8e7a8b3b68f3f87c164f58ad3da18c22073143de79f68e127a2eca9e51b60806502cd73d9aa9d728411edabc5d09e1160324358a0a5d7d307ba6b915c3c79942401f469c4cb0b0adcd25177c939768f110"
           ],
           "commitments":[
             "cm1uc8hl5umr8u7dgxxsrhp8e7rkwj8ue5qcjsuunfkt8s5dfld3grqwjnyvh",
             "cm1u0exutsg529akllpatxnmnsuzcjcyu5q7j9nfxe30d9dj5ntjgpsyvempu"
           ],
           "proof":"ozkp10wlufvmlyze6wk6zg8m5sghzl75zzuswa3yp4l5zs3k8ngsfhxqdkcg2x4l0f7eunqd2309hemc4u0lveqmgv38xemqsheu5xh8qvcxyuj4z5600k9dm6uxndcww40s3phecn4c3zs7hjakd7mlua23rqze7cpul8qjd2p3jf7g4yhy97ph5cmqp3vj8naa90zxpl678hentktuh6gc000kctmdl0k9xgmt39lstaf0kkew7aw0khvvyxfk8l46z8sqn79y26s834j4v9rsk2gdgw3pgy39gfl8jc7563wn2p76007qw8wzvsfahshktyeul2838qvhepfq7l3gg8kz97stekzy8unwvyausznnkpc88ms8nv9cyvxsm4ew5hg94e7xs5zgsfn047v8fl2jzs80jhf8g5c49u774rvuq7l2elcj7j0f2s58a9yqp6mlmwlm8emrsqqgcnmsq8",
           "serial_numbers":[
             "sn19my835fmg0yqte5pycgm9h3j7quvc2s6s8dmngvce0ew8xvvpvfqgqpsej",
             "sn17w9vn0n4hf0a038e8jhm4qjdcq2r2wp63sdu98cjmwxn3xl4kcpqa0jcvz"
           ],
           "transition_id":"as15d8a5nrc86xn5cqmfd208wmn3xa9ul3y9l7w8eys4gj6637awvqskxa3ef",
           "value_balance":-500000000000
         }
       ]
     }
   ],
   "id":"1"
}
```