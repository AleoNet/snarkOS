# Latest Block
Returns the block from the head of the canonical chain.

### Arguments

None

### Response

|        Parameter            |  Type  | Description |
|:---------------------------:|:------:|:-----------:|
| `block_hash`                | string | The hash of the block. |
| `header`                    | object | The block header containing the state of the ledger at the block. |
| `previous_block_hash`       | string | The hash of the previous block. |
| `transactions`              | object | The list of transactions included in the block. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "latestblock", "params": [] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
   "jsonrpc":"2.0",
   "result":{
      "block_hash":"ab1mkgcrwag6avnp0a0ntxr9y4lt7y2fq9t0tqglkkuysk6jg2rhqrqa9788l",
      "header":{
         "metadata":{
            "difficulty_target":18446744073709551615,
            "height":6160,
            "nonce":"5141009373721822405820512044696916600360683867327778483074084451822398628111",
            "timestamp":1636486761
         },
         "previous_ledger_root":"al1u0rtgkhsd94mucjmxsy3ze76xap67aj4hx4jgfysehaes4vrvu8s227n0d",
         "proof":"hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqp2ek5pandazcszav4fnt4nsutfgxfwy5sc508xhgrkfjsa6aur3l0cls9supd0mcuzd9l6mlswfmqf5ln5c7ylf52yynx5wy8fw478az4z6emxeu3f5p8375zzgk7sra3gujf3g7d3hsz0knq7qk87trwq6955kjkyq2dvqt0lztlwmjg39x6fn84w6fva6czxhnu5v2nnttzewdl0z908kqmt53u8m7mtcul5pep7lsplhgne6pdrtgz8c4p8d9rllky48ezm8waqta5lmtmk26g9pvkmqy4ke7dpp0z97f239pwcqqqcqqqqqqqqqqp8qvzj40qqg4hd4e30593sxj8725xfury39f67gfw359ufcx0t3kcue42phlgpvhf6akqlzfh5y5q2qdead436q0n5cpnh5jrpc2urnqfqxe56neyxkc55wgh8zyx0l29qpmhm567gp8as92wzw3p2vgwq3cvcdfltq45mle2c5pgz8dfw0sg7up9yf9wq6zxxu5mf3kplxs5c77ww097jp7l6pe55r0kw2gmqpqgqqqqqqqqqqqdm0n57r5w2qnpgggujfyrfv4vsyhulqfpmc7ujnxrrzw2ct8lusus7zx2m9rcce9y3xmamqhw29srtfh7waqzp3yta54f29uezsh40tjuncpe2z8kjlyjr77la8clt78qkg8lxwfvjr02xg58jj20d7wqqyqqqqqqqqqqq2lpumrtvvs8srlldr00c3qdwqhhdcpm8ufhuruzd26e0utjxnsr78umtcmge7peglwsf6d3adpwvq4u8ylddupuc66g6n2gtresw6qr940ywqxl9vdallel8ghyzutjh9y7mzxk6cdgj2mhra86vphep3949hdux84xyytydfx8mup5lyhqs2pyv6rg64lr772wg5hlzr43gyqvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqq59hacw5f6gcw39een200h0q7ahe5fpk5gm7f4h55ncmv5k0vzstzw8g0xgzc598wq2wswg5ea2mgqqgrw854s7zqncef060fxu5qvquf24gz73tqvww2mhrw2s3a5343puqf06zyj6kk5ltwvrxkrpamachqate665muvjvl5ayw3ncl7clyc4lj9y63xa3rvgawt72cd3hd2qqqqqd0jlq2",
         "transactions_root":"ht1w3s2c6mnc3e6crtkv40gfmmjvykxgh88ac6tv6cy7jucmsxlxsgql99vqh"
      },
      "previous_block_hash":"ab1frc9xutuzphcu2gqsvf29d755wdyztuulxhxt9h7d6xwck74zcfq4lv6u8",
      "transactions":{
         "transactions":[
            {
               "events":[
                  
               ],
               "inner_circuit_id":"ic14z3rtzc25jgjxs6tzat3wtsrf5gees5zel8tggsslzrfyzhxw4xsgdfe4qk6997427zsfl9tqqesq5fzw5q",
               "ledger_root":"al1enk2kwh9nuzcj2q9kdutekavlf8ayjqcuszgezsfax8qxn9k0yxqfr9fr2",
               "transaction_id":"at1wqa7u4unjc3p5nah9rgwrcy6cm0nxhykmtpaxlmkuj53v57q6q8sw7s8yp",
               "transitions":[
                  {
                     "ciphertext_ids":[
                        "ar1j0ak2ahmmhv58nnedk5klp0pvapy4pku30hkzx4j4mtxpnyuwqqqa7s5a2",
                        "ar1ac980j3ha20pwpqrh56pd4wd2vsqla93xq33lc52lvftt0t82qzsc4r48e"
                     ],
                     "ciphertexts":[
                        "4e74f8677f988b4a2a9f5a1d6d5ed17d8e9c94bc8de39cf60795b6722bacc5066d0743473d47e22cda0765f10556601200ff1b0234e005993b4a121a5c593a07218dfeea0512a1f8f9c40f15f5788998af5d9f0ff988d506631fc5418b024910941b9e696f68517d186c4ad45c6368c71c2bdaea3ecbf37f149e3491d94f260dce9a226095df1c5ab0b6a625b29a878bdeecf466299804b999c30182450d37128b8a91b27b63a028ffe64163b88efdb50a9ed554161be0ec59502678417050089cc5d3bc3092539581e0fb2885cb4eaa59f353a57c78763822716047a858e10c4081efa99d66d79076813216bd97d85d8be39ac07ecc056246c9bf087a20fd04d5c20f3cf5076ecd9f6f8cfeb22d5f0baaab5f628809049f36654d6df83a7c0ad47aef89b99b4fee12006c37a7eb921746135fee398654ad47ce969db19a2610",
                        "1f51b8a3f69bc9b30345cc782bfa95fd420b15c09f435b92ae450efc7c936d034783f67bdece49660b38e4f257b705f82377aff23680b6d3be7abb93628148083bfa001b40fc439da9f47f09f03f7c395956c5a6a9f548cd79d041cba780bb075144bd4d80d44dd54cf3405f5d82699195f73aa97c2d156147c33c262bdda80e54eedbab214a9a8cc067abb5ee1c4dff2aa0cb91c2978660267a1672680976011cdd073ad56eeefc871d590e8771a47ccf0ded9346b46627e48f99b1219fee00addeea3358c264228454ff3621422abe8910510ab69f66fba6a04c11f7a9c90c3bf93975827a1dd12a19f1d0d7c7bebf1e6b94f2028f197bc9627435586ed00120de877290afdb77927c9b381407bf35a7169aa4e489ec0886c5fc525e28c5031e1b8a4381cbe2a84ff995467109bb946b379b20879b282fa69307846a76a709"
                     ],
                     "commitments":[
                        "cm13marlfxutm46mltm9svuw3zz5ljvdqpv4qdcdemmlrytgj90xcxst2ezcl",
                        "cm1yc2fxyxf6yuvp9e8qt6r7gw05pzhavca3959muqef74360590ypsncn6jt"
                     ],
                     "proof":"ozkp1cljn66grygrsrfa0nv6ynmc83sf5myfqn2ndhtldnna6k3srwd3fq5sgf6vtlvzedccp7xh6zyua4jmm88kj0ga3kcfqnelteyuwxx9xh7sqlrgalcwrjt0td83ljsmxryw78ffwuh8fcph8772kxpnvspngk3q3f3zzzep4s7uldkw5ank7pux8r3cy0pxf4mhlr2g8vkydklq90z6lcvvkqpxjs7j4lelpa0y2g76whz0rgaca4xgqvf6x077wm72jzpzp6elrk68xxlc8ls4z0fvhpjnhs2dzqk8g9t26avjezzqux2jwz2r9sx2ttgjhu8xe2t9hmcpmx3d3tw20xxwchrs6vcn38m962kdxusuea8kv6n69m9rd3xf5ha0ykqwncw7p2mazhe63tg0yzjenskhuz7fwau720ut3tdmprlajtsg0r4drh8qjqtl7xfpuveusqqghqpqqk",
                     "serial_numbers":[
                        "sn1d4nmmksyuul63zmsf56p4maf6hd9edpaxjs4mglv6wfs8gmqjggsk8nucl",
                        "sn1ffck8gzh0gnsqsjujgsg6fpc8jfvfl8krhwceegs7gu6v6s9zgysh83rmr"
                     ],
                     "transition_id":"as1lwzgt7r0d22kewedvpwjexwypewjpwzq5e6383fxu0y0esft2yzqxhs6ek",
                     "value_balance":-150000000
                  }
               ]
            }
         ]
      }
   },
   "id":"1"
}
```