# Get Blocks
Returns up to `MAX_RESPONSE_BLOCKS` blocks from the given `start_block_height` to `end_block_height`.

### Arguments

|       Parameter      |  Type  | Required |                         Description                         |
|:--------------------:|:------:|:--------:|:-----------------------------------------------------------:|
| `start_block_height` | number |    Yes   | The block height of the first requested block in the array. |
| `end_block_height`   | number |    Yes   | The block height of the last requested block in the array.  |

### Response

|     Parameter         |  Type  |                Description               |
|:---------------------:|:------:|:----------------------------------------:|
| `result`              |  array | The array of requested blocks            |


#### Block

|        Parameter            |  Type  |                            Description                            |
|:---------------------------:|:------:|:-----------------------------------------------------------------:|
| `block_hash`                | string | The hash of the block.                                            |
| `header`                    | object | The block header containing the state of the ledger at the block. |
| `previous_block_hash`       | string | The hash of the previous block.                                   |
| `transactions`              | object | The list of transactions included in the block.                   |


### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getblocks", "params": [0, 1] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
  "jsonrpc": "2.0",
  "result": [
    {
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
    {
      "block_hash": "ab1a04ehlymquvlsuht7ssyh59p68z9249fla2dpque8rzke6s7gyqshxg4dn",
      "header": {
        "metadata": {
          "cumulative_weight": 1,
          "difficulty_target": 18446744073709551615,
          "height": 1,
          "timestamp": 1638274968
        },
        "nonce": "hn1h4wq7dfrzhvcz8l5wvqle54pur4mqc6tnpk3r897zam3dpk0hqxss8h6a2",
        "previous_ledger_root": "al1rwfdyp6xk39cuh2gnj6pxhndqy44tch93paudslwnvx5qcr3esyql9efpw",
        "proof": "hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqq9k3uuaywk8u46es2w5dvudg88gp9v287frk55ayszvl6wvjrynsmnmx2fx0yd5dxahee9v0qzsqqqn8yqfe0x7j7f7ckg7fdevycy99vhws87zskmrctfvyk48ra33kt8alreacccgxc8xp67ewjxgpcqwd0qrxtquzly66kghx2nqd4l5d0v72xdldelpj47xxjmjtctaf3ymjc7ehvw6k6n9rw25cht23rupng3ssecd454wrlpxq34ee7vjafv2dhxxlw49cs2ncvmplralazvhqe6lgcvm93sxtw0ulda505cszqcqqqqqqqqqqzwfkpkxrdnwgkfgxefeteg832ac9r448ht6ekas2ncgs89v4slauzwwh3puwcdryx2nng35rtxe9qfcsmrt5d5rqc94lvj8s87qjm8vzs7ajxjdd6kapsaqdeqjepv3d53tz24njhaq7wq5dc66fwuyr6q8rzht3thg92yvrlkckqjzers6v7ezhxvx77s7udlc43ttqprs7v7jeavns08jygmmvz00n4dfkvqpqgqqqqqqqqqqqe9ravrh3uqxfsyf5ej4dccml7ktjr2sugsft0aelrsw7hydhzajussav7m4a8ntdahw744ykx5nqre6c04vjt8k8e2wqgq4slgqg76lvtmnwzlyv6e2ltx8z7xuz6sphq9xurtgfywwwhjl70czajlxlqgyqqqqqqqqqqqylequa2vwvms9jt6p80ka6nj9s6wlnna79xy757xaj3kmwv8j7r6w3zcfcremkvser6hnm7n4v239utytk9v53rutd83kpv2cjd0lqanrvmm62yk5x20l9qn4wkmkxvwqdvp2t8d0tphzwqr809uy0w8qezc4gw8xkwth50n083vllax758ghhma9upt3p7drv79sj6fa2yc0qvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqq3v8ffmhfputspdfrp8lzymqyxwh8e7gvxeztfevdw9c27q67rg94knsxvkshz4ws9kdd7e8tx23cqq0vl8rc9trczszqmmlq6fqt52rwfvnt204lh9y9zlfldwjj8ncqqqkylx4d3862m2eetvt2kvq3uycfe8at8xzp55du4ks2s4pkwnrqj8azhgmf593y0fdqhnfuvxs7eqqqqqdz3n7e",
        "transactions_root": "ht1fa84m5qgqxm52p2sxfg3xvv5c0zjpvd0wr0g64w8ggnhdxd2gc8quc9gg8"
      },
      "previous_block_hash": "ab18946qsq2ppqylhk03ftpg7wjuknp4gwpqz0hhp8hl2ahn94sg5zqxd8qw8",
      "transactions": {
        "transactions": [
          {
            "inner_circuit_id": "ic13cstkmt5j4qqzfu5am8jx2rhxm0hqplyzcgzyueefz7n32xl4h53n4xmxvhjyzaq2c0f7l70a4xszau2ryc",
            "ledger_root": "al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
            "transaction_id": "at1sklmmp43ufyfev4rk9a8v68la2v29s8suplg2duxes7aju3c7uqs3qkw8k",
            "transitions": [
              {
                "ciphertexts": [
                  "recd1d4q7x6w0mgctwdh20qy2wn63d5dey07syr8vs7ka8fezfd395qppxguxktedhny8ndrmc7pfftu5mrj7agd342s9vz6vzccets60kplr3mpjzhhg7lwlpwj0fvhmp8vy4q49uy607zamxupcd65txzg2q38gxj0m6cdt5xdz0encr8wp9kt2rmeqe36yf4yrqgl5kssf0ypqv5mazvm3ltwlynuurzzw0lfdmxwfz5dqgtueltt5h0msn5k47rqjqgps6qx3pjk5sucz9jdrq322f25ghgk826tfy98pje7amrd5scxpea4dzq7t7wtgh6re5cdhp7wjupgpnpgfhw55l8hc6ax8dmtwcpeunngs68dlmq68vsh0jehlj7pw8zkyrsja3gmffatfmk6xw26szpkdgnxjjdvy3vahj6ppegl4dq7kcga6km84h2zzwds7lnccxufskd4fvfp",
                  "recd1rr50fxwrw2ztjx4rd432tqjr6updgy3ufn6q5vqsqemllevgtuxq9u8tugszkvvfdzpncwe7r3clfmms87z003v3ms2cg93cjlc6ypya0xyfv9hekah25wvar4y39ueg0dyzekmk07wk86tfzez7yfl4prh2laztfn86zqt43jefkrsa5525903x29jjz55nlxhlkjfvh68sn8ahaj4y4dxlwx9acps9xnak6yapzmw59cck2ft9a55mrvlhzpsj68tp560y27044207xae4va400prhpsn72ft84y2kmv8xuk6e45zx7kqvyudxesy83td59l7rfa498cn6mnxf389m7jfhrle0604v7zsr4caewgr5hnw0mqmrt5w0u3hhv5p0wm8faswgv40ddrya8kl6pk4ewp67yae3g2z6jyxtlc5tkdxkkt4l9hlu9jurhe3t02lmsjlqgjf8frt"
                ],
                "commitments": [
                  "cm1vlsf9jy46esuylztvz7pmf95kvjv39hh82knvlty9jplrcl0lsyspr3nqh",
                  "cm19ftu455wcrad2hzf92g8dzm7vvza3uuwjyc84wt9hv6pdz4gcsrqfy5m78"
                ],
                "events": [
                  {
                    "id": 1,
                    "index": 0,
                    "record_view_key": "rcvk1hd2ey0r8rj4958hz8tjmnguq0zqelnak0l04px5nl7fdk2du3yqq4fpdqq"
                  },
                  {
                    "id": 1,
                    "index": 1,
                    "record_view_key": "rcvk1rcuxgt9zy6cae8q2m35mhqchmeg8vthj8vrt732j8f7hvhx5lsys6n86v0"
                  }
                ],
                "proof": "ozkp1kgecjacsyaelc8cdltfzcwxyye9c6ulljrpsahtyyqh9g4swqyj7j4yy9vnw5zuc9xkssl56z2wh95s00szp09ma8ydh5uld8zr7ng2848l6sjppr9atll4d49wm46e4qc2hlvz8a45xa369073jd84usza20rrg5ax0p6q2w26zrzvlyat0fk6qv2u4c3glz9ey6x395h70jsu885wsu53rzypldhr6awh89pakuruvlc9wqs5j2phn38kncrkk2ct7a796kd5n6p6m2n20a8jnffg6p87wj3sc9g8t2cgzkha4u7qy0wg9lzwm9n6ekpy0c4ljs2gp06aat58mge4uqqkzag38crcyjpukzucxzpvyswrkzn0uxwfx05frx7xvtfn5yzq94xexnwemmtgcr3y66mc6ghtlacqmw2wfetdf7l6wlwcazucs4aw7txgdt7j6jkagqqgq5c480",
                "serial_numbers": [
                  "sn1ja4wheq75k5rtqu9ang77y4gmmr3xv6rmuj0zfrr4pntna7yl5pqudefjp",
                  "sn17nm844zsddvyp8q3ths8th3mchr3jc7s7r30gm6le485yc4tyqpq6k905x"
                ],
                "transition_id": "as1z796upddadydzww87v6udtdrpmj9m35029ehn4qxkej20ztjdg9smx65ek",
                "value_balance": -100000000
              }
            ]
          }
        ]
      }
    }
  ],
  "id": "1"
}
```