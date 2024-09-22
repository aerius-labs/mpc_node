# TSS Network

TSS Network is a robust implementation of Threshold Signature Scheme (TSS) for distributed key management and signing operations.

## Table of Contents

- [TSS Network](#tss-network)
  - [Table of Contents](#table-of-contents)
  - [Features](#features)
  - [Architecture](#architecture)
  - [Components](#components)
    - [Manager Service](#manager-service)
    - [Signer Service](#signer-service)
    - [Common Components](#common-components)
    - [Application data flow](#application-data-flow)
  - [Installation](#installation)
  - [Configuration](#configuration)
  - [Usage](#usage)
    - [Starting the Manager Service](#starting-the-manager-service)
    - [Starting the Signer Service](#starting-the-signer-service)
    - [API Endpoints](#api-endpoints)
  - [API Reference](#api-reference)
    - [Initiate Signing Request](#initiate-signing-request)
    - [Get Signature](#get-signature)
    - [How to test MPC](#how-to-test-mpc)
      - [Command to run test](#command-to-run-test)
  - [Security Considerations](#security-considerations)
  - [Contributing](#contributing)
  - [License](#license)

## Features

- Distributed key generation and management
- Threshold-based signing operations
- Secure communication between signers
- Fault tolerance and Byzantine fault resistance
- Integration with RabbitMQ for message queuing
- MongoDB storage for persistent data
- RESTful API for easy integration
- Comprehensive error handling and logging
- Metrics and monitoring support

## Architecture

The TSS Network consists of two main components:

1. **Manager Service**: Coordinates the signing process, manages signing rooms, and handles API requests.
2. **Signer Service**: Participates in the distributed signing process and interacts with the Manager Service.

![architecture](https://github.com/user-attachments/assets/c0a6cbc7-03e2-4928-b538-2d8e12b70c9f)

## Components

### Manager Service

The Manager Service is responsible for coordinating the signing process, managing signing rooms, and handling API requests. It is implemented in the following files:

rust:src/manager/service.rs
startLine: 1
endLine: 86

![manager_service](https://github.com/user-attachments/assets/7cc393f0-2181-4d64-a81e-46fe818c49dd)

### Signer Service

The Signer Service participates in the distributed signing process and interacts with the Manager Service. It is implemented in the following files:

rust:src/signer/service.rs
startLine: 1
endLine: 883

![signer_service](https://github.com/user-attachments/assets/30bc2c3c-0ada-41f2-901d-6882104e457e)

### Common Components

The project includes several common components used by both the Manager and Signer services:

rust:src/common/types.rs
startLine: 53
endLine: 92

### Application data flow
![data_flow](https://github.com/user-attachments/assets/07728f25-229d-49f7-98e1-8af6b867eef3)

## Installation

1. Clone the repository:
   ```
   git clone https://github.com/your-username/tss-network.git
   cd tss-network
   ```

2. Install Rust and Cargo (if not already installed):
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Install dependencies:
   ```
   cargo build
   ```

4. Set up MongoDB and RabbitMQ (refer to their respective documentation for installation instructions).

## Configuration

1. Create a `config` directory in the project root.

2. Create configuration files for different environments:
   - `config/default.toml`
   - `config/development.toml`
   - `config/production.toml`

3. Set the required configuration parameters in these files. Example:

   ```toml
   mongodb_uri = "mongodb://localhost:27017"
   rabbitmq_uri = "amqp://localhost:5672"
   manager_url = "http://127.0.0.1"
   manager_port = 8080
   signing_timeout = 30
   threshold = 2
   total_parties = 3
   path = "0/1/2"
   signer1_key_file = "signer1.store"
   signer2_key_file = "signer2.store"
   signer3_key_file = "signer3.store"
   ```

4. Set the `RUN_MODE` environment variable to specify the configuration to use:
   ```
   export RUN_MODE=development
   ```

## Usage

### Starting the Manager Service

To start the Manager Service, run:
```
cargo run --bin manager
```

### Starting the Signer Service

To start the Signer Service, run:
```
cargo run --bin signer
```


### API Endpoints

The Manager Service exposes the following API endpoints:

- `POST /sign`: Initiate a signing request
- `GET /status/<request_id>`: Get the status of a signing request
- `GET /signature/<request_id>`: Retrieve the signature for a completed request
- `GET /health`: Health check endpoint

For detailed API usage, refer to the [API Reference](#api-reference) section.

## API Reference

### Initiate Signing Request

**Endpoint:** `POST /sign`

**Request Body:**

```json
{
"message": "Message to sign" // Any string message to sign
}
```

**Response:**
```json
{
"request_id": "550e8400-e29b-41d4-a716-446655440000",
"status": "Pending"
}
```


### Get Signature

It will return signature when ```status``` is ```Completed```.

**Endpoint:** `GET /signing_result/<request_id>`

**Response:**
```json
{
  "request_id": "994ca821-8462-432a-a47e-97c898c8fe1b",
  "message": [
    83,
    117,
    110,
    105,
    108
  ],
  "status": "Completed",
  "signature": {
    "r": "ed5f91d15045f73ef7f1067b20f00914697cc09284deb72967ebe091b4e78f57",
    "s": "2fe1089e63086908dbf93b3ad43a6b672194ea94c53575a8a8210c01ccb04347",
    "status": "signature_ready",
    "recid": 1,
    "x": "e90afacf19e50498e886d2d2a5b22ca34ecfe0b3f063b8d7f1e5eabd37b5f8d8",
    "y": "aa5d7c0bbf991462d5884999b00b0826d1857dfdd6d07d0b7ce7fed47d5bbf77",
    "msg_int": [
      83,
      117,
      110,
      105,
      108
    ]
  }
}
```
### How to test MPC

Make sure these services are running locally

```
// MongoDB
"mongodb://localhost:27017"

// RabbitMQ
"amqp://localhost:5672"
```
#### Command to run test

```
cargo test --package tss_network --test manager_service_tests -- test_signing_flow --exact --show-output
```


## Security Considerations

1. **Key Management**: Ensure that private key shares are securely stored and never transmitted in plain text.
2. **Network Security**: Use TLS/SSL for all network communications between components.
3. **Access Control**: Implement strong authentication and authorization mechanisms for API access.
4. **Secure Configuration**: Keep all configuration files, especially those containing sensitive information, secure and separate from the codebase.
5. **Monitoring and Alerting**: Implement comprehensive logging and monitoring to detect and respond to any suspicious activities.
6. **Regular Audits**: Conduct regular security audits and penetration testing of the system.
7. **Dependency Management**: Regularly update and patch all dependencies to address any known vulnerabilities.

## Contributing

We welcome contributions to the TSS Network project. Please follow these steps to contribute:

1. Fork the repository
2. Create a new branch for your feature or bug fix
3. Make your changes and commit them with clear, descriptive messages
4. Push your changes to your fork
5. Submit a pull request to the main repository

Please ensure that your code adheres to the existing style conventions and includes appropriate tests.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
