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
  "jsonrpc":"2.0",
  "result":{
    "metadata":{
      "difficulty_target":18446744073709551615,
      "height":6164,
      "nonce":"7431854142636124133718475970690936399991195220046817601810838296960117666247",
      "timestamp":1636486834
    },
    "previous_ledger_root":"al1zd9fn8fj4wkkea895qztz0hyhq2fds836yqlnguj2n5f2gj4xcqq2mj02a",
    "proof":"hzkp1qvqqqqqqqqqqqpqqqqqqqqqqqr9qhz2qca83e0pl0agevaju2qymt5pwjx6kf4vkhdcs2rk0h7ynvv3sw2yhmswf8pppt7vqyflcfqgzjn4ppzpgesdwdzmn77ucn7930xdt40gp2vfp7z8t68304uqe4q6qr4xrsct9kghc9d77mqfv7xqrz2z8azhz2yuuewk6u0tfqafzj2x3uc7l5uy58ze30ustrjhpky0qk7pf72v6e4kyeel0veeagjvpjmjv6v9359zwc5c9gh2qqt5cs5vfquwysp83x6rsrha7t725p9eapssgaxqfllcq28yumj55655qqqcqqqqqqqqqqpuzgas23tfdk7vtdxdakj6nxeslzuj30jvjflwn0er2gt2pxd0jau9ua04fy0h8f4a7a0s3tlxdtqyu8rjty9s26j7eny5knc8cvsye8edeclmanjed53xan7jje020puru4jceg36gqf5u8whvjrvrpqqxh59ta4kluecprawpwk7vgg36eleq4kcfmjr57xjdnfegf5xvmjjx5qdccdayjx4yy0tjlafv6vvqqgqqqqqqqqqqqakf8kcu2r5yp5l2ce4dlyc8qrp4a5suyghfygcx2nghj9c50k3e5escs2dt43ndc5jky28z274pqpffk9ps6hljmyq80ew7eve5v2lkeucev8hh8r92uwpcy2ddf5kh5paer5kwdkcnk97acpaz2yfxfqqyqqqqqqqqqqqfemp9amjs8km44tdrqmm37w4nec5gvnxa9y55p58zsmpyr6yegpexktgdteykhc4akdkxym4w4u24lqd93haypjkansgu4s8ckgulqrllmscymc8kjya34fvgyyxf67q4tk4ktkgkvvskdsf55ha3yjzq63lea2y8wsyvpn9jk7vxdt38j0q4heh2z3wvw74hqfhnzhaa7csrqvqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqyqqqqqqqqqqqeef347jgsmz2mxamwyr2h2k53hqh30hln04pycukxsvf3fj7lphn7mg6nwrsgj3qwujs5gtlm4wsqq06vmujq8vufkfd6lthfjdwc56ydvlhq9jkz5xcagddhmny8km3qmdfxmls8kssqs0euzhqq8gj3n08a2myxl9q5m6d7zv9yyx5wy3m8lqzvqslzwhut5fm45szfjxztqgqqqandu5p",
    "transactions_root":"ht1glw36mv7q0sg46sag7da2w32k4amy7utglmrh6d3udy557t54qgs8va3uc"
  },
  "id":"1"
}
```