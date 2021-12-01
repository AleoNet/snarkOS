# Get Ciphertext
Returns the record ciphertext for the given commitment.

### Arguments

|   Parameter  |  Type  | Required |                     Description                    |
|:------------:|:------:|:--------:|:--------------------------------------------------:|
| `commitment` | string |    Yes   | The record commitment of the requested record ciphertext |

### Response

|   Parameter  |  Type  |          Description         |
|:------------:|:------:|:----------------------------:|
| `result`     | string | The record ciphertext. |

### Example Request
```ignore
curl --data-binary '{"jsonrpc": "2.0", "id":"documentation", "method": "getciphertext", "params": ["cm1xck4eyf3a3qnz69yyrr3jf698mqzwpjgkqu0j359p0sdr5wyjyqsn0604p"] }' -H 'content-type: application/json' http://127.0.0.1:3030/
```

### Example Response

```json
{
  "jsonrpc": "2.0",
  "result": "recd1v76mftwzagt9k9nsjjpdqgytv4ddk24e9q7f240daar7avcv3q9gd9rx6c230n99jhxfj24xpvkrr5vk04fl2kapa0a0a895hvevzq7tnwuat9lzwpy4c4rxys6uaj34098295t9fff7khqctvkcglumqlvg47rwzhqhw9u5zxfhug9dde67dyjc6uflp4x028mrmzkhfa6qn0l6jju8lfhmy5crcqqefjv8m4zwv34tvk03d65gdmv4fe35wtgy6rmy4heq89uwh0hqe40k2g7nyj2rk6xlgqnf724pt6ynkefxwypmvhhjzk806re4njej552jfq74ej0ykhrcxa93l9n6rkchlhuuzz2fpqtt2npqz8avnv442ng4djm8lve4dlqfelpjjn5yj425rs98pvn5k54gvn5vku3wek3ytxe8zpen7n2saf060j97u8yyygt4y9zqklnek3v",
  "id": "1"
}
```