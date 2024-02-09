#!/bin/bash

# Enable ports 4133/tcp and 3033/tcp
sudo ufw allow 4133/tcp
sudo ufw allow 3033/tcp

# Enable ufw and reload

sudo ufw enable
sudo ufw reload

# Check ufw status

sudo ufw status