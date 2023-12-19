# Migration Scripts

This directory contains scripts that are used to migrate the database from one version to another.

## Instructions


#### Assumptions

The scripts identify nodes by searching for `Host snarkos-n{NODE_ID}` in the SSH config file.
```
Host snarkos-n0
  HostName 192.168.1.1
  User ubuntu
  IdentityFile ~/.ssh/{YOUR_PEM_FILE}.pem
  Port 22
```

In addition, the scripts assume that the `.aleo/` folder is located at `~/.aleo`.

### 0. `load-snapshot.sh`
