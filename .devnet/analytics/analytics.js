const fs = require('fs');
const axios = require('axios');

// Read the ~/.ssh/config file
const sshConfigFile = fs.readFileSync(`${process.env.HOME}/.ssh/config`, 'utf8');

// Define the AWS node name to search for (e.g., aws-n1)
const awsNodeName = 'aws-n1';

// Use regular expressions to extract the IP address associated with aws-n0
const regex = new RegExp(`Host\\s+${awsNodeName}[\\s\\S]*?HostName\\s+(\\S+)`);
const match = sshConfigFile.match(regex);

if (match && match[1]) {
    const ipAddress = match[1];
    const baseUrl = `http://${ipAddress}:3033/testnet3/block`;

    console.log(`IP Address: ${ipAddress}`);
    console.log(`Base URL: ${baseUrl}`);

    // Function to get the latest block height
    async function getLatestBlockHeight() {
        try {
            const response = await axios.get(`${baseUrl}/height/latest`);
            const latestHeight = response.data;
            console.log(`Latest Block Height: ${latestHeight}`);
            return latestHeight;
        } catch (error) {
            console.error('Error fetching latest block height:', error.message);
            return null;
        }
    }

    // Function to calculate the average block time
    async function calculateAverageBlockTime() {
        const latestHeight = await getLatestBlockHeight();
        if (latestHeight === null) {
            return;
        }

        let totalBlockTime = 0;
        let previousTimestamp = 0;

        for (let height = 1; height <= latestHeight; height++) {
            // Print the current height every 10 blocks
            if (height % 10 === 0) {
                console.log(`Processed ${height} blocks...`);
            }

            try {
                const response = await axios.get(`${baseUrl}/${height}`);
                const timestamp = response.data.header.metadata.timestamp;

                if (timestamp && timestamp > 0) {
                    if (previousTimestamp > 0) {
                        const deltaTimestamp = timestamp - previousTimestamp;
                        // Skip outliers (to account for stopping the devnet and restarting it)
                        if (deltaTimestamp < 500) {
                            console.log(`Block ${height} Delta Timestamp: ${deltaTimestamp}`);
                            totalBlockTime += deltaTimestamp;
                        } else {
                            console.log(`Block ${height} Delta Timestamp: ${deltaTimestamp} (skipped)`);
                        }
                    }
                    previousTimestamp = timestamp;
                }
            } catch (error) {
                console.error(`Error fetching block at height ${height}:`, error.message);
            }
        }

        const averageBlockTime = totalBlockTime / (latestHeight - 1); // Subtract 1 for the first block
        console.log(`Average Block Time: ${averageBlockTime} seconds`);
    }

    // Calculate and output the average block time
    calculateAverageBlockTime();
} else {
    console.error(`No IP address found for ${awsNodeName} in ~/.ssh/config`);
}
