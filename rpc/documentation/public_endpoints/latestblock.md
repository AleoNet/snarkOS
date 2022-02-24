# Latest Block
Returns the block from the head of the canonical chain.

### Arguments

None

### Response

|       Parameter       |  Type  |                            Description                            |
|:---------------------:|:------:|:-----------------------------------------------------------------:|
|     `block_hash`      | string |                      The hash of the block.                       |
|       `header`        | object | The block header containing the state of the ledger at the block. |
| `previous_block_hash` | string |                  The hash of the previous block.                  |
|    `transactions`     | object |          The list of transactions included in the block.          |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"1", "method": "latestblock", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
  "jsonrpc": "2.0",
  "result": {
    "block_hash": "ab18946qsq2ppqylhk03ftpg7wjuknp4gwpqz0hhp8hl2ahn94sg5zqxd8qw8",
    "header": {
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
    "previous_block_hash": "ab1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq5g436j",
    "transactions": {
      "transactions": [
        {
          "inner_circuit_id": "ic13cstkmt5j4qqzfu5am8jx2rhxm0hqplyzcgzyueefz7n32xl4h53n4xmxvhjyzaq2c0f7l70a4xszau2ryc",
          "ledger_root": "al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
          "transaction_id": "at1ky80ktk2tcyytgg3dvg3jqtu64kc6nzdrwg75nv0c6u78grkh5qqdu804w",
          "transitions": [
            {
              "ciphertexts": [
                "recd1v76mftwzagt9k9nsjjpdqgytv4ddk24e9q7f240daar7avcv3q9gd9rx6c230n99jhxfj24xpvkrr5vk04fl2kapa0a0a895hvevzq7tnwuat9lzwpy4c4rxys6uaj34098295t9fff7khqctvkcglumqlvg47rwzhqhw9u5zxfhug9dde67dyjc6uflp4x028mrmzkhfa6qn0l6jju8lfhmy5crcqqefjv8m4zwv34tvk03d65gdmv4fe35wtgy6rmy4heq89uwh0hqe40k2g7nyj2rk6xlgqnf724pt6ynkefxwypmvhhjzk806re4njej552jfq74ej0ykhrcxa93l9n6rkchlhuuzz2fpqtt2npqz8avnv442ng4djm8lve4dlqfelpjjn5yj425rs98pvn5k54gvn5vku3wek3ytxe8zpen7n2saf060j97u8yyygt4y9zqklnek3v",
                "recd1u5chlqz8n80rwem25de9npujv2uh006yajgyum9p5kn4rsu9s5ymgrwgle39pz87s0726g4rg47dx5nl330680gxmyxffyg7p77qvppfql3p3hxncp9fpus8upsa5nlfwfnck7k4hzcjskrnrfza6tqcpgvquuv663ahswju6s3wcawh9ktz87ewzgpj2nc8gc9wd30zc8zsgu5xyen4q352u7y6l985kv2hq6nx9hu4n4mhgglacw7dc026y6qglwh0l302gwxs0s804waax472h4tv2npmprtvp5hkzg7hhm360squhgnxtpdthh0ncyrdklqy57nlfr6z5dm080xd2z9uw3h9fpu9vqsy9q4vakw00wk0prwf92ekmnh9e00v4l2a4sldmcnzcj90p75nqlrd5ek80e6l3xz559meskjeq7kpyhftsxcptc9d009xuh6nxlyszq7uktv"
              ],
              "commitments": [
                "cm1xck4eyf3a3qnz69yyrr3jf698mqzwpjgkqu0j359p0sdr5wyjyqsn0604p",
                "cm1up0j5cq0k3w96skhsq750m6alw8dcau5msn390h8fpkgny5zdvps9h9dp8"
              ],
              "events": [
                {
                  "id": 1,
                  "index": 0,
                  "record_view_key": "rcvk1mujt98tc2r04l58haxjv48s5a7vnhx8ws24fxpdruuk3z37vscqsjtvlg5"
                },
                {
                  "id": 1,
                  "index": 1,
                  "record_view_key": "rcvk1yuqvyczasq876gjt8xwz7d2dxzs3umlp8nccpcg3nmlp4qxs35yqgflhy6"
                }
              ],
              "proof": "ozkp1fhewv363wgl04jcxdnmaznkeszy2svj6ncxr9pl5f5l3lm6mgsr8e6tqxpkhhtxc6pesfd40hxfgwz7luqwwa00uwzu5s8jfq9n743n4y4dldf9htr20jv9zpw59cf4xxwurnpckq0wt8r5hfdn5m2d9qryk20yz9zfeyvv7hrexxvd707qx730q2qeppnu70y0q3rpnqzprtxrclgqptrwlx2cdzg5ywkayn8f04xelpge4d73a3tmyvlyuj5phlv5lxq2afh4zaxnxw8f2e5k32xu9w0vmq7xldqmyv7pxjfj2mzqrwyagg7nzsay34kx2zutx33r0eugfqgtqlhrzrnhqu2npk0kxwcx27rgvpfcwemsns56d7xn0zety5mkcje3ud0usjfhmdwhh3eypzh0x3svs5jhm9nhtpqc7j7ms3gu4rc7d352g42fzv2vvv5lsxuygzqgxrha3j",
              "serial_numbers": [
                "sn1m70m3egkxqq5dmalym3hf5arz296k37h87kv4ztge48c3a6hmcysw22avz",
                "sn1q8y49taxgquprav54nkd42n8dd8egj0rghjfg834q0zlfv3p9cpst9mkj5"
              ],
              "transition_id": "as1xuppwj4x7eswxppdlg3pt49vue2kwfw3cw8m8k3uqxe5e7945g9s4s8lz5",
              "value_balance": -1000000000000000
            }
          ]
        }
      ]
    }
  },
  "id": "1"
}
```