# Latest Block Header
Returns the latest block header from the canonical chain.

### Arguments

None

### Response

| Parameter               |  Type  | Description |
|:-----------------------:|:------:|:-----------:|
| `metadata`              | object | The block header metadata. |
| `previous_ledger_root`  | string | The Merkle root representing the blocks in the ledger up to the previous block. |
| `transactions_root`     | string | The Merkle root representing the transactions in the block. |
| `proof`                 | string | Proof of Succinct Work. |

#### Block Header Metadata

|      Parameter      |  Type  | Description |
|:-------------------:|:------:|:-----------:|
| `difficulty_target` | number | Proof of work algorithm difficulty target for the block. |
| `height`            | number | The height of the block. |
| `nonce`             | number | Nonce for solving the PoW puzzle. |
| `timestamp`         | number | The block timestamp is a Unix epoch time (UTC) (according to the miner). |


### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblockheader", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response
```json
{
  "jsonrpc": "2.0",
  "result": {
    "metadata": {
      "difficulty_target": 18446744073709551615,
      "height": 0,
      "nonce": "605189531586914990276139803763573111245566376445087174167030943959527940992",
      "timestamp": 0
    },
    "previous_ledger_root": "al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
    "proof": "hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqqdtf45rfuetk7swgcn03h4q2mcpu7vakgfz4w7y2g37lfyvw0zraerguvut37ydajmh0kh2s2xt8q8cvqarnl47cr9zm2e4rrjcjwpq4zhaslamjraky7xkr3jl9llgpf7g695t4v6up6f8qsmtcu5gtvqa60nkcjkfyfdvzjkt3lct5rxrewetkpgh0r460t68qed0s994rc7rdc3u7svjxmrs5eud4l4st8yqhreeqtc2c5u7au7n6jk6kdpjwezvaytnmkklpjs6sfwffsee7h6d5txdpmzw2rhlagfst4fkxsngzqcqqqqqqqqqqrhtgwync0a7cnhh4nl6grmqs4qahzq704pfhgsdsh0mrzlqt22w8uxykuyn8kw953vx2yjzk90ayqp7l28u0hgvgq5y8etlg07gj06hacrwyhe9lkvkde698243zuvrjhhnry97s9pegqx66qrpng3vpxqaty5gu6cwrdthmqjluyhl70x08dta9c27wsvjserrvezf7s9mgaf60x9a4a6kcyh2qc2z4eq7xzspqgqqqqqqqqqqqve93e6t5ftqhrfku4g4tey59hztng0tke5leyr5rjzwnqvd0eegusz9rtpccl5ky6kp28cft7nksxujtwg8ut00u6newqqqp0gdhpu0kpmmfnevfzy2r3z7ma734vk8c457epffq0xx8ycfxfpcmfenfqgyqqqqqqqqqqqr7396rqsutqvyjd0l6jvum0vvl9w7agvr2ef5tkuruqtgywevzp6k08gwh57lmf4xk6jg8k8n29hnqdet3kxwttq8aqfh072jzclxz9cjdsqzz4aw57aeyp8gwu4m4wghcgu8nr9qel2sc55866rawqwqssy6uqxqy3uax8a8ul5fpvttcyf3d6u0wda5rfc04lydclhctjcpqvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqqcdwhfdar5rudlxqf4mllawtuu47vg6dgt38hyakkg25hc23jpqqmkmp3egt3h0rlundyugccsewgzqgs87yhdnev6lz97dae3yz5dvwylf39fmns5gd97asvzrhkjp68q520zgzmuw7cgheg5t9nn62u4j473t8p4f97daps76u4gu4f7ldm0jpk5pphxzv9fw44y89w0zkayqqqqqwqqnkx",
    "transactions_root": "ht1s3hd25d25zxfvnsxa789w594n7qarzuzm49n6h23mr5yhzh7ysgq426nk5"
  },
  "id": 1
}
```