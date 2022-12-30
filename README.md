### Whatsapp Manager


### Purpose
This application has the intention to abstract the communication through whatsapp in order to be used on multiple application without the setup needed.


### Types of messages

You can request three types of message

- Text
- Button Reply
- List

### Example requests

- **Send a plain text message**

curl --request POST \
--url http://localhost:8080/message \
--header 'Content-Type: application/json' \
--data '{
"system_id": 1,
"to": [
"56936748406"
],
"message_type": "text",
"content": {
"body": "Test message"
}
}'


- **Send a reply button option**

curl --request POST \
--url http://localhost:8080/message \
--header 'Content-Type: application/json' \
--data '{
"system_id": 1,
"to": [
"56936748406"
],
"message_type": "button",
"content": {
"body": "Test message",
"buttons": {
"title": "Buttons",
"choices": [
"btn-1",
"btn-2"
]
}
}
}'


### Systems IDS

0 -> ALL SYSTEMS
1 --> Whatsapp Manager
2 --> META API


