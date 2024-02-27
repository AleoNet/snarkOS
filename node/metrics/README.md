# snarkos-node-metrics

[![Crates.io](https://img.shields.io/crates/v/snarkos-node-metrics.svg?color=neon)](https://crates.io/crates/snarkos-node-metrics)
[![Authors](https://img.shields.io/badge/authors-Aleo-orange.svg)](https://aleo.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](./LICENSE.md)

The `snarkos-node-metrics` crate provides access to metrics for the `snarkos` node.

## Instructions

#### Quick Start

To start up Grafana and Prometheus, run the following command:
```bash
cd node/metrics
docker-compose up --detach
```

To check that the metrics are running, go to http://localhost:9000.

Lastly, go to [http://localhost:3000/](http://localhost:3000/) to see the metrics dashboard.
The initial login is `admin` and the password is `admin`.

#### First-Time Setup

1. **Start snarkOS with Metrics Enabled**
    - Launch snarkOS using the command line with the `--metrics` flag to enable metrics tracking.

2. **Navigate to Metrics Directory**
    - Change your current directory to `node/metrics` using the command `cd node/metrics`.

3. **Deploy Prometheus and Grafana with Docker**
    - Execute `docker-compose up --detach`. This command uses the `docker-compose.yml` file to set up two containers: Prometheus and Grafana, eliminating the need for direct installation.

4. **Verify Metrics Accessibility**
    - Use the command `curl http://localhost:9000` to check if the metrics are accessible at the specified URL.

5. **Access Grafana Dashboard**
    - Open your web browser and navigate to `http://localhost:3000`. This is the Grafana user interface.

6. **Grafana Login Process**
    - Log in using the default credentials: username `admin` and password `admin`. On first login, you'll be prompted to change the password, but you can choose to skip this step.

7. **Configure Prometheus Data Source**
    - In Grafana, navigate to `Datasources`.
    - Select `Prometheus` as the data source.
    - Enter `http://prometheus:9090` as the server URL.
    - Confirm the setup by clicking `Save and Test`. You should see a message confirming successful connection to the Prometheus API.

8. **Import snarkOS Dashboard**
    - Return to the Grafana home page by clicking `Home` in the top breadcrumb navigation.
    - Click on the arrow next to the `+` icon in the top right corner.
    - Select `Import dashboard`.
    - Drag and drop the `node/metrics/snarkOS-grafana.json` file into the top panel of the import interface.
    - From the dropdown box, choose the Prometheus data source you previously set up.
    - Finalize the process by clicking `Import`.

Following these steps will successfully set up and configure a monitoring environment for your snarkOS nodes using Docker, Prometheus, and Grafana.
