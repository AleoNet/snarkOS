# Instructions

## 0. Create EC2 instances

Start by creating EC2 instances in the AWS console.
- Ubuntu 22.04 LTS (not Amazon Linux)
- Security Group - Inbound Policy
  - SSH - Port 22 - 0.0.0.0/0 (or your IP)
  - Custom TCP - Port 3030 - 0.0.0.0/0 (or your IP)
  - Custom TCP - Port 4130 - 0.0.0.0/0
  - Custom TCP - Port 5000 - 0.0.0.0/0

Be sure the give the EC2 instances a name tag, i.e. `devnet`.

Make sure you set the correct SSH `.pem` and have the `.pem` in your `~/.ssh` directory.

## 1. `config.sh`

This script generates the SSH config for the EC2 instances.

#### Install `jq`

This script requires `jq` to be installed.

On macOS, you can install `jq` with Homebrew:
```bash
brew install jq
```

On Ubuntu, you can install `jq` with apt:
```bash
sudo apt install jq
```

#### AWS Credentials

This script requires the `aws` CLI.
[Install `aws`](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
(For macOS, there is an installer PKG in the link)

Then run:
```bash
aws configure
```
You will be prompted to enter the following information:
- AWS Access Key ID
- AWS Secret Access Key
- Default region name (e.g., us-east-1)
- Default output format (e.g., json)
This information will be used to authenticate your AWS CLI requests.

To get your `ACCESS_KEY` and `SECRET_KEY`, go to AWS IAM dashboard and follow these steps:
1. In the IAM dashboard, click on "Users" in the left navigation pane and then click the "Add user" button.
2. Click the "Next: Permissions" button.
3. Click the "Attach existing policies directly" button.
4. Search for "AmazonEC2FullAccess" and select it.
5. Click "Next" and click the "Create user" button.
6. Select the user and click "Create access key".
7. Select "Command Line Interface (CLI)".
8. Click "Create Access Key".
9. Copy the "Access key" and "Secret access key" and paste them into the `aws configure` command.

Try running:
```bash
aws sts get-caller-identity
```
If you get an error, you may need to wait a few minutes for your IAM user to be created.
If you get a response like the following, you are good to go:
```json
{
    "UserId": "XXXXXXXXXXXXXXXXXXXXX",
    "Account": "123456789012",
    "Arn": "arn:aws:iam::123456789012:user/your-iam-username"
}
```

## 2. `install.sh`

This script installs snarkOS on clean EC2 instances.

This script assumes you have ran `config.sh` and copy/pasted the output into `~/.ssh/config`.

This script will:
- Clone snarkOS
- Install Rust
- Install dependencies
- Install snarkOS

## 3. `reinstall.sh`

If you are actively developing, you can use this script to reinstall snarkOS on the EC2 instances.

This script will fetch the latest changes from the Github branch that you specify, and reinstall snarkOS.

## 4. `start.sh`

This script starts snarkOS on the EC2 instances.

## 5. `monitor.sh`

This script monitors the EC2 instances.

#### Switch Nodes (forward)

To toggle to the next node in a local devnet, run:
```
Ctrl+b n
```

#### Switch Nodes (backwards)

To toggle to the previous node in a local devnet, run:
```
Ctrl+b p
```

#### Select a Node (choose-tree)

To select a node in a local devnet, run:
```
Ctrl+b w
```

#### Select a Node (manually)

To select a node manually in a local devnet, run:
```
Ctrl+b :select-window -t {NODE_ID}
```

#### Exit (But Keeps the Devnet Running)

To exit the monitor, run:
```
Ctrl+b :kill-session
```
Then, press `Enter`.

## 6. `analytics.sh` (optional)

This script generates analytics for the EC2 instances.

To run this optional script, you must have Node.js installed.

## 7. `stop.sh`

This script stops snarkOS on the EC2 instances.

## 8. `clean.sh`

This script stops snarkOS on the EC2 instances, and removes the ledger DB from the EC2 instances.
