# Get Block Header 
Returns the block header for the given the block height.

### Arguments

|    Parameter   |  Type  | Required |                   Description                  |
|:--------------:|:------:|:--------:|:----------------------------------------------:|
| `block_height` | number |    Yes   | The block height of the requested block header |

### Response

| Parameter               |  Type  |                   Description                |
|:-----------------------:|:------:|:--------------------------------------------:|
| `metadata`              | object | The block header metadata. |
| `nonce`                 | string | The nonce for Proof of Succinct Work.
| `previous_ledger_root`  | string | The Merkle root representing the blocks in the ledger up to the previous block. |
| `proof`                 | string | Proof of Succinct Work |
| `transactions_root`     | string | The Merkle root representing the transactions in the block. |

#### Block Header Metadata

|      Parameter      |  Type  | Description |
|:-------------------:|:------:|:-----------:|
| `cumulative_weight` | number | The cumulative weight of the chain at this block. |
| `difficulty_target` | number | Proof of work algorithm difficulty target for the block. |
| `height`            | number | The height of the block. |
| `timestamp`         | number | The block timestamp is a Unix epoch time (UTC) (according to the miner). |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblockheader", "params": [0] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
  "jsonrpc": "2.0",
  "result": {
    "metadata": {
      "cumulative_weight": 0,
      "difficulty_target": 18446744073709551615,
      "height": 0,
      "timestamp": 0
    },
    "nonce": "hn1lrw8l2w7m74caxxngv57ps28ng49gdn77tgq26sm78qhxz9qwcyqc9djn4",
    "previous_ledger_root": "al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
    "proof": "hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqqwpcgtfx3p968slxsgm0ej54pyp46df56ufnmwz3d2rjajg2fj62hq5en9nnx2cdnrww8d9nw9qdqf67m8ctxnk6xyk8tu5gthjjd48f085wvua32px9mw29g7ee2562wjt2fz2ncklpgefeefrfmv90xqgha5r42svfn9nktlpjvtezspwm2l4ejg7kqyydg5s5hgcq2c6taxxtlz4yww43dz0gvxdj4uex9sq83dp9hq3kgcfvgu47mquptj20cyzu0jdksqq2annt72wg4ysfskpsj0kayf9m6t73xs8qhwczrscqqcqqqqqqqqqqqj5d5uw7jep7kmnssfjv3utj97d2pghy0u3wjmlj750dv5y3wc9axdhvj3qs60u9wfl96d2r39qqq9ek8fy8dckr6an58arfrawnuemmqw464rud6r50t8x5cxfn9lhtj6jtma82rahw4zmv2emt8r5h5qfvnd34p93ejxrn6rph8r9ul8tfe65l8t4pvxnpltmcxjnn8wsgvr3d3qc7n39y8wlwz3f0kn3z8yqqgqqqqqqqqqqp2lrqkejgksx2vasrwssl4plmt05vk9he22x0k5uca6k0kehyda6cxu5vrj0q4qqe75elekpj086qpcjqrvgq5tk7cp6q9dj8g0unzvwwg8c5459ug3h3k6r077s8lrh3z63s262jfa5y528f0nnmc89wqgyqqqqqqqqqqqyfekfkkzzgr94sdfzkvyzgh3kag03evgmczdyteaaznnjgla87y3959mxv74c2pk3k5fs3hx4lcgwajgjn8h5x7aj2fc7mns9qegmzr9d8ljnrlwvzwhu0f7r9czwevwq3zeevkkt2033yexr55d3v93swzke3tvcdmlqyrsaey46wqxcvm4l7gzxvlca9a8p7wcls4a9mccsqvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqqm8jv4psnamnzyqqwp9svtmekgysu5gcns7tk4rf2x9g6vay36afuhszk4kyd3t9m72g3kgf57y8gzq00zpcr7l9lyulduxxqe0658j2kyct252hhmvdt7u9t9029z7cazxr40dcx8glm3xg429v2tly3fm70mkmcu56r37czya0elz79cjcfuap9qzgpjd2y6ng6h7uyflzxvqqqqqf9tevv",
    "transactions_root": "ht1gl4pv2jw4vyjtdrxn4806vttajn3k3fm2yrfe8akt36zqs72psxsv8rw4c"
  },
  "id": 1
}
```