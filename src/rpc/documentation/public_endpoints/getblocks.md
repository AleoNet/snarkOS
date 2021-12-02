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
   "jsonrpc":"2.0",
   "result":[
      {
         "block_hash":"ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh",
         "header":{
            "metadata":{
               "difficulty_target":18446744073709551615,
               "height":0,
               "nonce":"605189531586914990276139803763573111245566376445087174167030943959527940992",
               "timestamp":0
            },
            "previous_ledger_root":"al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
            "proof":"hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqqdtf45rfuetk7swgcn03h4q2mcpu7vakgfz4w7y2g37lfyvw0zraerguvut37ydajmh0kh2s2xt8q8cvqarnl47cr9zm2e4rrjcjwpq4zhaslamjraky7xkr3jl9llgpf7g695t4v6up6f8qsmtcu5gtvqa60nkcjkfyfdvzjkt3lct5rxrewetkpgh0r460t68qed0s994rc7rdc3u7svjxmrs5eud4l4st8yqhreeqtc2c5u7au7n6jk6kdpjwezvaytnmkklpjs6sfwffsee7h6d5txdpmzw2rhlagfst4fkxsngzqcqqqqqqqqqqrhtgwync0a7cnhh4nl6grmqs4qahzq704pfhgsdsh0mrzlqt22w8uxykuyn8kw953vx2yjzk90ayqp7l28u0hgvgq5y8etlg07gj06hacrwyhe9lkvkde698243zuvrjhhnry97s9pegqx66qrpng3vpxqaty5gu6cwrdthmqjluyhl70x08dta9c27wsvjserrvezf7s9mgaf60x9a4a6kcyh2qc2z4eq7xzspqgqqqqqqqqqqqve93e6t5ftqhrfku4g4tey59hztng0tke5leyr5rjzwnqvd0eegusz9rtpccl5ky6kp28cft7nksxujtwg8ut00u6newqqqp0gdhpu0kpmmfnevfzy2r3z7ma734vk8c457epffq0xx8ycfxfpcmfenfqgyqqqqqqqqqqqr7396rqsutqvyjd0l6jvum0vvl9w7agvr2ef5tkuruqtgywevzp6k08gwh57lmf4xk6jg8k8n29hnqdet3kxwttq8aqfh072jzclxz9cjdsqzz4aw57aeyp8gwu4m4wghcgu8nr9qel2sc55866rawqwqssy6uqxqy3uax8a8ul5fpvttcyf3d6u0wda5rfc04lydclhctjcpqvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqqcdwhfdar5rudlxqf4mllawtuu47vg6dgt38hyakkg25hc23jpqqmkmp3egt3h0rlundyugccsewgzqgs87yhdnev6lz97dae3yz5dvwylf39fmns5gd97asvzrhkjp68q520zgzmuw7cgheg5t9nn62u4j473t8p4f97daps76u4gu4f7ldm0jpk5pphxzv9fw44y89w0zkayqqqqqwqqnkx",
            "transactions_root":"ht1s3hd25d25zxfvnsxa789w594n7qarzuzm49n6h23mr5yhzh7ysgq426nk5"
         },
         "previous_block_hash":"ab1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq5g436j",
         "transactions":{
            "transactions":[
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
            ]
         }
      },
      {
         "block_hash":"ab1zfygptd2x8hacsgjeew39fnpurahvqhdj7qe4kc050rhzrdclcqsgng7rk",
         "header":{
            "metadata":{
               "difficulty_target":18446744073709551615,
               "height":1,
               "nonce":"2344093721547332605145783223163543945492690438135730928763977704336941446492",
               "timestamp":1636365241
            },
            "previous_ledger_root":"al10lg7n89s4tw7gy6wylgz0upjyr76y0mmj5azaqplnf3kkpksnyrsgyzsvg",
            "proof":"hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqql2yyfx8ag3ucak6ehrth73nrapcnts37972lx5mj9u7exldwxyn93ayfyam8yga05k3data0mj8q2pewkfcl4skgu05zxd2axt2yz9zmzv7qu444kzwy7vnycr8ckxmjflkyfpvld2v5sxuajndv8l5zq9a5p0u94apswhh0hgnzx0yeyp7uywqunwvt4lmswr3h4e80aqadpyjqaec9a3xz0gr8elcy6avg5qz0dmhcpdngrgjc3w2k65nnpfs4daleauhu9v2648xx3hrenfjv2vv6lk7jvns48xq52mr4kqdg4qzqcqqqqqqqqqqz8m6zm5ktzkmgw486kpwxecdyl8v5h8m0jq7hepcdvz6zlcrzsngjpsz78kqh327ju7wvml4yux4qyep2rj2k6cdev5a0she6647dpujuzgfp5gyvskslut3x43snhz3hx2vwlj3y0vkcn9e6ugh67q7jqyk3txyxcd3vyzf8x47cndf603qgtfqnstl8rndug4zdtp6hmkq62cf4de2kcemgh6cqczyqcpexqqqgqqqqqqqqqqpdazwae42l2gj6effvypkjutxfae5z7mw88klq4nkqdfq8dta3667lqt8dhs7fwv7hf36elak8gnqydsx5wyfkpsvmjsw6h0l9nxzvvmzqakqqrgsz9csyj28ujh8y6areqnv97g75nqk6m5327w58z5mqqyqqqqqqqqqqqvg54kyvpaax6y8leleaku02ml8phymmnlqqh7rnvhs52q3505cz2gtqmh46v8usw05jhau99cwp95p9eug6e29732ez732q0kdsstqpk003pkkc8rqr9r8aeuarxucs54nrey00nurcs0sgdvqs3kv2239tw94pz69d484zsl40vu525kjv9wmd6q55sgqfp7ne604ajdjacrqvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqqxdjdelz4wyw4msdxgh3hrwxn9u4djtc5qkv8r4z4zldkvv784xfqqsek22a9edhznr53ul35tl2gqqtgfnvvj4m5p5dkaf7h72hr66q8pc4m75ecd0vg6mzdqjy383xdpth632kq4eh4mpw2ry0xcynxt9vyt00s68n2pwy4fmz47xlfxqlje6rgm4r6e2rgmlf2apystzxanqqqqqmr79ky",
            "transactions_root":"ht1guett649e4p5c40ryc4hkkknydeklvy8lxf9pl2fp3vatfxufsgsz37txh"
         },
         "previous_block_hash":"ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh",
         "transactions":{
            "transactions":[
               {
                  "events":[
                     
                  ],
                  "inner_circuit_id":"ic14z3rtzc25jgjxs6tzat3wtsrf5gees5zel8tggsslzrfyzhxw4xsgdfe4qk6997427zsfl9tqqesq5fzw5q",
                  "ledger_root":"al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
                  "transaction_id":"at1z64pyrke88rv4p5lrxvg3fnfkdldqfmnpykltyrkfazen494wq9sq00xmz",
                  "transitions":[
                     {
                        "ciphertext_ids":[
                           "ar1xutprtfu3s5ry25qtq5e5tx0ty93q2wjx86z464wncmtcfatnqpspr4jf4",
                           "ar1vgu3y87pwejdgphxcc3dxewx7pw5xzfg636v7vw5unr39fwdqsysgg69yy"
                        ],
                        "ciphertexts":[
                           "2dd1c371d12442ac625dafd0f4351246bb63f9bd35aecd3906bbf9a816af450a024878dae7622544b13c43e587cbf727ea1bd08949b4fdd345a95770a95b1b09c2beffbabeb61d628a6afc4ccd1da13342db1292b499995bacc98d623cecaa02570aa8b8336543c43c455e644efbcf4285d5c5dfb00d8cc33afea96e5bc7a40cd6ee9c7e66d76a9f481868edc7a3c1c3c2674825e61c1fa90b5a4c85126b4c096b34e043366bcded806555caaf4cc12bce3577923d2f8739f6985cb45f34da0817df60eb30105d5236d28e292e2697752c07957a3ab5229ea3bbc048ad3edd054475d272074b2cd5150c3c6b20a3f9c3af323832edb44e83ad88c2c898b9740753155dacaf61582f90e19f2e5c46d8fdf36b5bb98785f77388cbef0ff77d6a015015429ddc54142ad1fa5e4ad2a5e2bd900d96f2e1c841fad8395100bf867711",
                           "57323f60abc6d2804b208bd5d4b575a633c2c839ab89ca6802047ac3556eea051cf4eee04cd6d9ada0bd2d2bf35a8da8260379750985f266cb8dd7ae2be126082cce89f9f6ad0fcd6a4a60fa7eb51abd565921d421e21ade61d320f2e72a2e039aa8f9eb9e8b9b5ea3e4e59b07e4142999571a0e5f77ad904fde2cf4b5c7df0bf7d0d21bec3991ecc5261fc439140b035dfec5cc3e4be0df440a25cc7583c60bd3398605934fb44da74f9ddc7d4a8d6275414c2526de574afce90cd5c4799908adf87aaf3c5dca8655fec0db883d336fa3457ab8109858ec4045ff55703dcf00eef7a20651357618da83eb5ec00ddbb7cec292f5940d2ed666dc457fc53a5a00ec5c287cd7679173b3f5656f4ce4ad797fc9a48ee7ea3dafbca600ae58ac4807d9478b45244c606b1f2f4b325a1f501b1d8fa5a204118871d7a33018bf0bfe11"
                        ],
                        "commitments":[
                           "cm10ltsfr78zgnxzvcq2j0fcc357p7tre5r0044yacffszzymutpvpqu7v97h",
                           "cm1p7ymcl6knmssl3l0kkl0ldsanrv7udfqudh5n9s2sf9h3gefw5zspdfegq"
                        ],
                        "proof":"ozkp1jcn3k9r4zgg2ce9zct4e6gnv9tx2h36qkhmnsj45ywsv0rhgpjjncf6zyjl9dwz5ssklzk4p2l9ql24x3hr3j70c64awvy672fyk3yl00fx77gxdqp9n9n28u5gx88rhxnza9ar8wyh85kprkqxcmscvqzms6grd0q7rn8nmhraqqz976hw4vr8v6srrmmrrqys8tpuzp07jzsannk08qdws78v4fsk6fasn6kvr0z59l2w42ufrfgk8h4h8rxj5f9ny5yg2jps8gj9h3d9ydc7pk8nyyatmykv750hzt2gkmq4ajsqz7xvge6sqps8vmm5pp4gpqhr0nzr5jelwn75nwtdndshxxpjqh8hjf34dxskv7mt042gdpyx95rs0a54xk6ue0666z04cz90rs0khfu0ra64g3q72cuhtwp4z0aa7ufgw7ahyx7skh8y45n8pdeqrka9cqqgtuge6m",
                        "serial_numbers":[
                           "sn1dzwyjdy294255fm7mrm0jvz3t773zl4l3q7ftp6ddxdvs60pyg8qrhdtj0",
                           "sn1cm2wwkwsgq5fs7m4gapfxfutf7qkr5pkvxd2v2z9anryhmqyfvys6r7j0f"
                        ],
                        "transition_id":"as1q2f5e47qm0rq0v8ertuyv3rdsq690rzn6dkcxma49cnsy95t4qys23608d",
                        "value_balance":-150000000
                     }
                  ]
               }
            ]
         }
      }
   ],
   "id":"1"
}
```