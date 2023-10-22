const fs = require('fs');
const axios = require('axios');
const yargs = require('yargs');

// Dim text style escape code
const dimStart = "\x1b[2m";
const dimEnd = "\x1b[0m";

// Function to fetch block data
async function fetchBlockData(baseUrl, height) {
    try {
        const response = await axios.get(`${baseUrl}/${height}`);
        return response.data;
    } catch (error) {
        console.error(`Error fetching block at height ${height}:`, error.message);
        return null;
    }
}

// Function to calculate the average block time
async function calculateAverageBlockTime(baseUrl, latestHeight) {
    let totalBlockTime = 0;
    let previousTimestamp = 0;

    for (let height = latestHeight; height >= 1; height--) {
        const blockData = await fetchBlockData(baseUrl, height);
        if (!blockData) {
            continue;
        }

        const timestamp = blockData.header.metadata.timestamp;

        if (timestamp && timestamp > 0) {
            if (previousTimestamp > 0) {
                const deltaTimestamp = Math.abs(timestamp - previousTimestamp);
                // Skip outliers (to account for stopping the devnet and restarting it)
                if (deltaTimestamp < 500) {
                    console.log(`Block ${height} - ${deltaTimestamp} seconds`);
                    totalBlockTime += deltaTimestamp;
                } else {
                    console.log(`Block ${height} - ${deltaTimestamp} seconds (skipped)`);
                }
            }
            previousTimestamp = timestamp;
        }

        // Calculate and log the average block time thus far
        const blocksProcessedSoFar = latestHeight - height + 1;
        if (blocksProcessedSoFar > 1) {
            const averageBlockTimeSoFar = (totalBlockTime / (blocksProcessedSoFar - 1)).toFixed(1);
            console.log(`${dimStart}Average Block Time Thus Far - ${averageBlockTimeSoFar} seconds${dimEnd}\n`);
        }

        // Print the current height every 10 blocks
        if (height % 10 === 0) {
            console.log(`Processed ${blocksProcessedSoFar} blocks...\n`);
        }
    }

    const averageBlockTime = totalBlockTime / (latestHeight - 1); // Subtract 1 for the first block
    console.log(`Average Block Time: ${averageBlockTime} seconds`);
}

// Function to calculate the number of rounds in each block
async function calculateRoundsInBlocks(baseUrl, latestHeight) {
    for (let height = latestHeight; height >= 1; height--) {
        const blockData = await fetchBlockData(baseUrl, height);
        if (!blockData) {
            continue;
        }

        // Extract the subdag object and get the number of keys
        const subdag = blockData?.authority?.subdag?.subdag;
        const numRounds = subdag ? Object.keys(subdag).length : 0;

        console.log(`Block ${height} - ${numRounds} rounds`);
    }
}

// Main function to fetch block metrics
async function fetchBlockMetrics(baseUrl, metricType) {
    // Function to get the latest block height
    async function getLatestBlockHeight() {
        try {
            const response = await axios.get(`${baseUrl}/height/latest`);
            const latestHeight = response.data;
            console.log(`${dimStart}Latest Block Height: ${latestHeight}${dimEnd}`);
            return latestHeight;
        } catch (error) {
            console.error('Error fetching latest block height:', error.message);
            return null;
        }
    }

    const latestHeight = await getLatestBlockHeight();
    if (latestHeight === null) {
        console.error('Unable to fetch latest block height, try again...');
        return;
    } else {
        console.log(``);
    }

    if (metricType === 'averageBlockTime') {
        calculateAverageBlockTime(baseUrl, latestHeight);
    } else if (metricType === 'roundsInBlocks') {
        calculateRoundsInBlocks(baseUrl, latestHeight);
    } else {
        console.error('Invalid metric type. Supported types: "averageBlockTime" or "roundsInBlocks".');
    }
}

async function main() {
    // Define command-line options
    const argv = yargs
        .options({
            'metric-type': {
                alias: 'm',
                describe: 'Metric type to fetch (averageBlockTime or roundsInBlocks)',
                demandOption: true,
                choices: ['averageBlockTime', 'roundsInBlocks'],
            },
        })
        .argv;

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

        console.log(`${dimStart}IP Address: ${ipAddress}${dimEnd}`);
        console.log(`${dimStart}Base URL: ${baseUrl}${dimEnd}`);

        // Fetch and output the specified block metric
        fetchBlockMetrics(baseUrl, argv['metric-type']);
    } else {
        console.error(`No IP address found for ${awsNodeName} in ~/.ssh/config`);
    }
}

// Run the main function
main();
