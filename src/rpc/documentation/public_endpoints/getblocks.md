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

|        Parameter            |  Type  | Description |
|:---------------------------:|:------:|:-----------:|
| `block_hash`                | string | The hash of the block. |
| `header`                    | object | The block header containing the state of the ledger at the block. |
| `previous_block_hash`       | string | The hash of the previous block. |
| `transactions`              | object | The list of transactions included in the block. |


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
      "block_hash": "ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh",
      "header": {
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
      "previous_block_hash": "ab1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq5g436j",
      "transactions": {
        "transactions": [
          {
            "events": [],
            "inner_circuit_id": "ic14z3rtzc25jgjxs6tzat3wtsrf5gees5zel8tggsslzrfyzhxw4xsgdfe4qk6997427zsfl9tqqesq5fzw5q",
            "ledger_root": "al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
            "transaction_id": "at1pazplqjlhvyvex64xrykr4egpt77z05n74u5vlnkyv05r3ctgyxs0cgj6w",
            "transitions": [
              {
                "ciphertexts": [
                  "580a3b830d17818b866c8f4f7bc4f518dcd775adce1274e902d07edc49e3e9005418da88939bac4437a271cb34cf792e3bad3385fe05ec086b5e5c485b291209a7ced29b1f6f169dd13504a71ce57c3348fe8778c7a102e26791fb298a7a0a0edfae8e065599cd3869d08a91ffa06fb6e41f68033d9b8d9269fa6f948f167f07c7d57ab6f3198c3d568a60525cb4959df7655dc8de9751d3cd8ec6b5aa834c0b0b081c6734fb62670b80cc885210392e6f56cd2b4aec22c53257a3422fe474041540a950c102b00c36cb6e889a7cefe2822a003e9332f3536a18fd3bc1690e11094593d25543c3df7e2fc4a24754eafdf4e0002ed4673c1a84c49ddcc510870efef1b4f95f0cf7d1a2e0e754eaa188a22cefb4938e616506154135091874c10ed4abe8bf94915a782e545acc07663237c2c826fd2c516fd14a2f9d2c570afe0d",
                  "3bba8e8ac95b43c3bc4be5792cf73f05458113641d7b15946e169188a7534f0ff6b7328b7549d042dfaeea13b81f8ea6eac72d68ea68681874a823fd08fa44043f074b48823758688ab1114ab0b5f2143ceaf1d8ef448ab21fd874a4e3ffb30919b8cddae76870d0e766fb43e847cb6f02fc18041e709695f9b03ffea5d7f206977a06b37c5a0f8b1a576b351233446bfaf6f8cc310bfcc1fa1138c310a3810d98f6cba62e95230e7966b59d3e383ab87a9f61760922b538e81004e6ab7efb02d811a7198af891db3eaa2d4e589643cf1d0f3c2b87c39d5d2cf163a59728b001d3e23485afe77843d2e3ab8554face8e7a8b3b68f3f87c164f58ad3da18c22073143de79f68e127a2eca9e51b60806502cd73d9aa9d728411edabc5d09e1160324358a0a5d7d307ba6b915c3c79942401f469c4cb0b0adcd25177c939768f110"
                ],
                "commitments": [
                  "cm1uc8hl5umr8u7dgxxsrhp8e7rkwj8ue5qcjsuunfkt8s5dfld3grqwjnyvh",
                  "cm1u0exutsg529akllpatxnmnsuzcjcyu5q7j9nfxe30d9dj5ntjgpsyvempu"
                ],
                "proof": "ozkp10wlufvmlyze6wk6zg8m5sghzl75zzuswa3yp4l5zs3k8ngsfhxqdkcg2x4l0f7eunqd2309hemc4u0lveqmgv38xemqsheu5xh8qvcxyuj4z5600k9dm6uxndcww40s3phecn4c3zs7hjakd7mlua23rqze7cpul8qjd2p3jf7g4yhy97ph5cmqp3vj8naa90zxpl678hentktuh6gc000kctmdl0k9xgmt39lstaf0kkew7aw0khvvyxfk8l46z8sqn79y26s834j4v9rsk2gdgw3pgy39gfl8jc7563wn2p76007qw8wzvsfahshktyeul2838qvhepfq7l3gg8kz97stekzy8unwvyausznnkpc88ms8nv9cyvxsm4ew5hg94e7xs5zgsfn047v8fl2jzs80jhf8g5c49u774rvuq7l2elcj7j0f2s58a9yqp6mlmwlm8emrsqqgcnmsq8",
                "serial_numbers": [
                  "sn19my835fmg0yqte5pycgm9h3j7quvc2s6s8dmngvce0ew8xvvpvfqgqpsej",
                  "sn17w9vn0n4hf0a038e8jhm4qjdcq2r2wp63sdu98cjmwxn3xl4kcpqa0jcvz"
                ],
                "transition_id": "as15d8a5nrc86xn5cqmfd208wmn3xa9ul3y9l7w8eys4gj6637awvqskxa3ef",
                "value_balance": -500000000000
              }
            ]
          }
        ]
      }
    },
    {
      "block_hash": "ab1r7yn5khy3q7pch0zkgdt34xv6k0fwmdx4krzkyq4xv3fv8jg5q8sd2nmq9",
      "header": {
        "metadata": {
          "difficulty_target": 18446744073709551615,
          "height": 1,
          "nonce": "3239127888019359521940590329882854030117464324755981128795956159677250090177",
          "timestamp": 1636713935
        },
        "previous_ledger_root": "al10lg7n89s4tw7gy6wylgz0upjyr76y0mmj5azaqplnf3kkpksnyrsgyzsvg",
        "proof": "hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqqet96rsum59pt9frxtp33cf26n3tmxs59cky8yp8xw54j4pl2xrqsalcxd6r0snktpz77auk2czxqgn0zmaduucnzjrtv8c0wfmgk84cywysy9p8903p0lu6se4ld3v522fs0pqw72hrezw7vqv7gd74sqvwf270yft6fsfgepq70dd7dy7ys3nn54yj4wwagq8w8vfsh6fl726u2s5qe683lkdpqjvk5mlttvqtzsw586qh0d8llqdhx49fc3gnnsg9w9pj9jty8dmmtr4sc2ymjny4d3sk4mchryvr29ppkwcyyeqzqcqqqqqqqqqqqmrwthnlcnx9lvf3p4uvkdpkjhh9pw5yzm47hmn6kvyc53lpxnkq5uev7mmy69etzd2frle4jh9nqy6vjytkzwmvp9hxx4vactvfrgqt4gn6yymktvc587e3s52vna5agsvmzr6h3u0r3j2f23vghjarkqvhht0nw59gytnptacdh5wtp9tpem9hpw9w8c6t2456lpzrrt8uzpy4mwtavmq5vjj4k7fa5p5m7cqqgqqqqqqqqqqp3l780n9jv3k28sucgu0846vyjdy67npdjelz4ngxa6hq580kh9jdh25wgmdafap2zplc7ku772jqqzuvdaqjqjtg0h5kdudm0mgmspnvczmurld4wty8dan5kvy8pccp7mt5vrw9efsvvh3m44qdw8kvqqyqqqqqqqqqqq2lmue6u8pn4nv9ryffxuky0qc6627gwlz7zq98m9mhcgwm9zvkremg40gdtfj36guzyawsk0qmse7tecaa6vjlnq698hvgdh6566cpxcc7h5qd6mk2cjzrpkt8yvr0ylq9wj0r8kxkrmf9crgftck6edqt5wul97zaelshrctcjl7f0nlqjglc9rhyq29rm6sgfaqrq7hnjg9qvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqquhjdfwmvrt2znj9mfuzzkwyzuq0nmztvej0wgafsfmarya8pxmv5cgu3nlu8qpu0ddfkc0u4w7kqzq2k6jg5aq3msu393pe0vyp048gqvgq8gktlwmkpfzkpgd0sg6evqhrhfparrprctpjq0pdm4vsy2hy8p5u4exvfy9vgts2wm3nzeg6vpntg6a9kx675mduc30pc9m5reqgqqq84eppp",
        "transactions_root": "ht1mqtt44jlurraz5h3g6phxva5a24zevtk7lhhdf8s8ttpq4frmggq7qn6n6"
      },
      "previous_block_hash": "ab1h6ypdvq3347kqd34ka68nx66tq8z2grsjrhtzxncd2z7rsplgcrsde9prh",
      "transactions": {
        "transactions": [
          {
            "events": [],
            "inner_circuit_id": "ic14z3rtzc25jgjxs6tzat3wtsrf5gees5zel8tggsslzrfyzhxw4xsgdfe4qk6997427zsfl9tqqesq5fzw5q",
            "ledger_root": "al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
            "transaction_id": "at19pk9vcrwh8k8emypk9gx2zn6lsd43frn74gtgfcpechk2qvgyqys8ufph5",
            "transitions": [
              {
                "ciphertexts": [
                  "a095b00a6b5460c4eb295a84add45dc19a26c5a60955fc8006ef04809d6b2b0a95e2d0bc69e5abc5ae8efcb32d85c03758eac4ab37ba6536114f6c59294ed90cafdb6fc5f255ac74da43e67d5fe8cd942f91a1e33d36303e7bd41a6828b1cd11cb53764427bda7e9ff5fb876990f38e2204c11c9736215bfc21dc57b1c0b6e03aad8e56a55fa4e314134cb02377f9b83baaf19008eab657109b20d45c4aafd10eac63afb71a0d717129407c8555e25d34f3b80b59a506054e29f308d4485060c6b350dcf670f4d3dd7e22f7850c74d7ea1d79a2da743192e9c842ccd61bc721179d65eb2b3d2fb1ad1660a023ed4197b46ded2c648fafd321df8456b5b3b6f0e4db4d0b2eba03ad1396c8a985239e4715366a02458f495217884ae0adeebf611c16a9c18bbefeee278a33c44dc3915e6e66ab472d6b1927a862c6800db76d611",
                  "06862973484e3b63cdfc5cc66aa8ba48cf695ffd10e8e79301bea07d568b26097d160bdc338c8b195d78de4d5f27dc095e00eadf341388ceb776177338e17b0fb26df9be1b2ebae2411f1c8fb34bdc3cdd689f8bacc519581cd878d27ecca902cf5d385a796f7c9b3818a2c8b2e7eaf2c2434d409fd36e286b7c5d23e63809072cd860c691e52acec84a95d90fd1bd669a0fc479293bc96b899f0d4aca9ff10abc59f22669fc3e986ecf9203fadd60a0b79a27fedd2d47876e66ecd2274d230580a862b8b39f7d0647a74d1cd674a87e0f04a98a685212173760a2998b247b109ba40c08d8cf76335d926c66ac7378d405fda3156b803c2d0b9e8dbb41d7570f9239a0d74130c68b077b912525382abebc88b5d222baee6c3aff3d5ef7d42d11adb416cc3236d7b8734cbc0e6a039491b0628f55c28a2841aa26964ee88cb711"
                ],
                "commitments": [
                  "cm1jryt2uza3rhtjrm55nt4kg05aph0aus39nanlc0ckk4t7gvt0ugqxqwmgc",
                  "cm17hp0npw6xa9kpq86kt9afp89d249lzfksjlje5tczxgcz9q2hg8skxf4tp"
                ],
                "proof": "ozkp1s5cpvlnqerc4f7f8pq2x2j6cl22wdwnuhjug2erejq29mqw4frxtup35hhqevjqtnd6dr0hvvwta4ps93hzkj5h55md0vk9rdkhhm558sw3vkk7zwpj6m0zg0tx5nmjysqsqp4elwgpaczyq55us23s8qxwh43sk6l5lsdwucu40a7fh57jzllye70y4y75aj56lqzmzwf8hexd50nva66gjn3k5gcvsuq4kvm6g7hdmk28txqd992rr6clvzkry3p6smlndd3cn5m7jaqt2qyf3p2kw9frsgtlqpcy4p5mzst6sxcqdg3zrfwsfpgdxzhdnu03lesklvragye770carayzw6elju0zz67ul86gfg9c0sykl7ldwrkh2ydr2dhz7s39gh3dtelmqy9at2qvh5745nkzx2xujdrwlkgkhtzu4lwvn6lfch50mta4txwg8an32j3xgqqg44lh3q",
                "serial_numbers": [
                  "sn1wvwfy003qzne0zaahpugw6n3449p7z93xg4uux5re7c9evz4cgqqrx4n6k",
                  "sn1aqq7gh4n8ufkpxtwl9rxw05r3cxj8l0e2vv5zhy9crqyuapuuuyswq2cds"
                ],
                "transition_id": "as1mxpwn230r2gywfarjulp2am52pvg79qsazsla0alr0xu493duuqss6gee8",
                "value_balance": -150000000
              }
            ]
          }
        ]
      }
    }
  ],
  "id": "1"
```