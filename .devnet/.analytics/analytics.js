const fs = require('fs');
const axios = require('axios');
const yargs = require('yargs');

// Dim text style escape code
const dimStart = "\x1b[2m";
const dimEnd = "\x1b[0m";

// Function to get the IP address of a given AWS node
async function getIPAddress(awsNodeName) {
    // Read the ~/.ssh/config file
    const sshConfigFile = fs.readFileSync(`${process.env.HOME}/.ssh/config`, 'utf8');

    // Use regular expressions to extract the associated IP address
    const regex = new RegExp(`Host\\s+${awsNodeName}[\\s\\S]*?HostName\\s+(\\S+)`);
    const match = sshConfigFile.match(regex);

    if (match && match[1]) {
        return match[1];
    } else {
        console.error(`No IP address found for ${awsNodeName} in ~/.ssh/config`);
    }
}

// Function to get the count of AWS nodes based on the naming convention aws-nXX in the SSH config file
async function getAWSNodeCount() {
    // Read the ~/.ssh/config file
    const sshConfigFile = fs.readFileSync(`${process.env.HOME}/.ssh/config`, 'utf8');

    // Regular expression to match all aws-nXX formats
    const regex = /Host\s+(aws-n\d+)/g;
    let match;
    let highestNumber = -1;

    // Iterate over all matches and find the highest number
    while ((match = regex.exec(sshConfigFile)) !== null) {
        const nodeNumber = parseInt(match[1].replace('aws-n', ''), 10);
        if (nodeNumber > highestNumber) {
            highestNumber = nodeNumber;
        }
    }

    // Return the count of nodes, adding 1 because it starts from 0
    return highestNumber >= 0 ? highestNumber + 1 : 0;
}

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

async function checkBlockHash(networkName, blockHeight) {
    const numNodes = await getAWSNodeCount();
    console.log(`Detected ${numNodes} AWS nodes... \n`);

    for (let i = 0; i < numNodes; i++) {
        // Define the AWS node name to search for (e.g., aws-n1)
        const awsNodeName = `aws-n${i}`;
        // Get the IP address of the AWS node
        const ipAddress = await getIPAddress(awsNodeName);
        // Define the base URL for the node
        const baseUrl = `http://${ipAddress}:3030/${networkName}/block`;

        // Fetch the block data
        const blockData = await fetchBlockData(baseUrl, blockHeight);
        if (blockData && blockData.block_hash) {
            console.log(`${awsNodeName} - Block ${blockHeight} - ${blockData.block_hash}`);
        } else {
            console.log(`${awsNodeName} - Block ${blockHeight} - No block hash found`);
        }
    }
}

// Main function to fetch block metrics
async function fetchBlockMetrics(metricType, optionalBlockHeight, networkID) {
    // Derive the network name based on the network ID.
    let networkName;
    switch (networkID) {
        case 0:
            networkName = "mainnet";
            break;
        case 1:
            networkName = "testnet";
            break;
        case 2:
            networkName = "canary";
            break;
        default:
            throw new Error(`Unknown network ID (${networkID})`);
    }

    // Function to get the latest block height
    async function getLatestBlockHeight(baseUrl) {
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

    // Define the AWS node name to search for (e.g., aws-n1)
    const awsNodeName = 'aws-n1';
    // Get the IP address of the AWS node
    const ipAddress = await getIPAddress(awsNodeName);
    // Define the base URL for the node.
    const baseUrl = `http://${ipAddress}:3030/${networkName}/block`;

    console.log(`${dimStart}IP Address: ${ipAddress}${dimEnd}`);
    console.log(`${dimStart}Base URL: ${baseUrl}${dimEnd}`);

    const latestHeight = await getLatestBlockHeight(baseUrl);
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
    } else if (metricType === 'checkBlockHash' && optionalBlockHeight) {
        checkBlockHash(networkName, optionalBlockHeight);
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
                describe: 'Metric type to fetch (averageBlockTime, roundsInBlocks, or checkBlockHash)',
                demandOption: true,
                choices: ['averageBlockTime', 'roundsInBlocks', 'checkBlockHash'],
            },
            'block-height': {
                alias: 'b',
                describe: 'Block height to examine for checkBlockHash metric',
                type: 'number',
            },
            'network-id': {
                alias: 'n',
                describe: 'Network ID to fetch block metrics from',
                demandOption: true,
                type: 'number',
                choices: [0, 1, 2],
            }
        })
        .check((argv) => {
            // Check if metric-type is checkBlockHash and block-height is provided
            if (argv['metric-type'] === 'checkBlockHash' && (isNaN(argv['block-height']) || argv['block-height'] == null)) {
                throw new Error('Block height is required when metric-type is checkBlockHash');
            }
            return true; // Indicate that the arguments passed the check
        })
        .argv;

    // Fetch and output the specified block metric
    fetchBlockMetrics(argv['metric-type'], argv['block-height'], argv['network-id']);
}

// Run the main function
main();
