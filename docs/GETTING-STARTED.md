# Getting Started

A complete walkthrough of a 3-node Raft cluster with the OpenBao and AWS
Secrets Manager default plugins configured. We create an application, register
principals using three identity methods (API key, TPM, and AWS IAM role),
authenticate, and run OSL put/get against the OpenBao backend. The same OSL
calls work against AWS Secrets Manager by changing the `backend_ref`.

## Contents

- [Overview](#overview)
- [Conventions](#conventions)
- [Raft cluster setup](#raft-cluster-setup)
  - [Start all three nodes](#start-all-three-nodes)
  - [Initialize the cluster](#initialize-the-cluster)
  - [Add nodes as learners](#add-nodes-as-learners)
  - [Verify the learners](#verify-the-learners)
  - [Upgrade learners to voters](#upgrade-learners-to-voters)
- [System setup](#system-setup)
  - [Initialize the system](#initialize-the-system)
  - [Unlock the nodes](#unlock-the-nodes)
- [Application setup](#application-setup)
  - [Create an application](#create-an-application)
  - [Create a principal](#create-a-principal)
    - [API key](#api-key)
    - [TPM](#tpm)
    - [TPM activation](#tpm-activation)
    - [AWS IAM role](#aws-iam-role)
  - [Grant the principal access to the application](#grant-the-principal-access-to-the-application)
- [Authentication](#authentication)
  - [Request an auth challenge](#request-an-auth-challenge)
  - [Authenticate and obtain a short-term token](#authenticate-and-obtain-a-short-term-token)
    - [API key authentication](#api-key-authentication)
    - [TPM authentication](#tpm-authentication)
    - [AWS IAM role authentication](#aws-iam-role-authentication)
- [Working with secrets (OSL)](#working-with-secrets-osl)
  - [Put a secret](#put-a-secret)
  - [Get a secret](#get-a-secret)

## Overview

The end-to-end flow is:

1. Bring up the Raft cluster (start nodes, add learners, promote to voters).
2. Initialize and unlock the system.
3. Create an application and register one or more principals.
4. Grant the principal access to the application.
5. Authenticate as the principal to obtain a short-term token.
6. Use that token to call the OSL secret endpoints.

## Conventions

The examples use placeholder values that you should replace with the values
returned by earlier steps. You can export them as shell variables so the curl
commands can be pasted as-is:

| Placeholder         | Where it comes from                                   |
| ------------------- | ----------------------------------------------------- |
| `$ROOT_TOKEN`       | `root_token` returned by `/system/init`               |
| `$APP_ID`           | `app_id` returned by `/app/create`                    |
| `$PRINCIPAL_ID`     | `principal_id` returned by `/app/user/create`         |
| `$CREDENTIAL_ID`    | `credential_id` returned by `/app/user/create` (TPM)  |
| `$USER_API_KEY`     | `token` returned by `/app/user/create` (API key)      |
| `$CHALLENGE_ID`     | `challenge_id` returned by `/app/user/auth/challenge` |
| `$SHORT_TERM_TOKEN` | `token` returned by `/app/user/auth`                  |

All cryptographic material in the TPM and AWS examples (public keys, quotes,
signatures, ARNs) is illustrative. Substitute your own.

## Raft cluster setup

### Start all three nodes

Run each in its own terminal:

```sh
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
cargo run --bin decodering-server -- --id 2 --addr 127.0.0.1:21002
cargo run --bin decodering-server -- --id 3 --addr 127.0.0.1:21003
```

### Initialize the cluster

```sh
curl -X POST 'http://127.0.0.1:21001/raft/init' \
  --header 'Content-Type: application/json' \
  --data '{
  "raft_init": []
}'
```

### Add nodes as learners

```sh
curl -X POST 'http://127.0.0.1:21001/raft/add-learner' \
  --header 'Content-Type: application/json' \
  --data '[2, "127.0.0.1:21002"]'

curl -X POST 'http://127.0.0.1:21001/raft/add-learner' \
  --header 'Content-Type: application/json' \
  --data '[3, "127.0.0.1:21003"]'
```

### Verify the learners

```sh
curl -X POST 'http://127.0.0.1:21001/raft/metrics'
```

```json
{
  "osl_version": "1.0.0",
  "status": "raft-metrics",
  "message": "Raft node metrics",
  "data": {
    "running_state": { "Ok": null },
    "id": 1,
    "current_term": 1,
    "vote": {
      "leader_id": { "term": 1, "voted_for": 1 },
      "committed": true
    },
    "last_log_index": 6,
    "committed": { "leader_id": 1, "index": 6 },
    "last_applied": { "leader_id": 1, "index": 6 },
    "snapshot": { "leader_id": 1, "index": 4 },
    "purged": { "leader_id": 1, "index": 2 },
    "state": "Leader",
    "current_leader": 1,
    "millis_since_quorum_ack": 0,
    "last_quorum_acked": 1780215757178426002,
    "membership_config": {
      "log_id": { "leader_id": 1, "index": 6 },
      "membership": {
        "configs": [[1]],
        "nodes": {
          "1": { "addr": "127.0.0.1:21001" },
          "2": { "addr": "127.0.0.1:21002" },
          "3": { "addr": "127.0.0.1:21003" }
        }
      }
    },
    "heartbeat": {
      "1": 1780215757177625002,
      "2": 1780215757172533794,
      "3": 1780215757172533793
    },
    "replication": {
      "1": { "leader_id": 1, "index": 6 },
      "2": { "leader_id": 1, "index": 6 },
      "3": { "leader_id": 1, "index": 6 }
    }
  }
}
```

### Upgrade learners to voters

For the full set of `change-membership` options, see the API reference.

```sh
curl -X POST 'http://127.0.0.1:21001/raft/change-membership' \
  --header 'Content-Type: application/json' \
  --data '{
  "AddVoters": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" },
    "3": { "addr": "127.0.0.1:21003" }
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "raft-membership",
  "message": "Raft membership changes",
  "data": {
    "log_id": { "leader_id": 1, "index": 8 },
    "data": "Noop",
    "membership": {
      "configs": [[1, 2, 3]],
      "nodes": {
        "1": { "addr": "127.0.0.1:21001" },
        "2": { "addr": "127.0.0.1:21002" },
        "3": { "addr": "127.0.0.1:21003" }
      }
    }
  }
}
```

## System setup

### Initialize the system

Creates the root user and stores the plugin credentials. This can only be run
once. To update credentials later, use `/system/plugin/config`.

```sh
curl -X POST 'http://127.0.0.1:21001/system/init' \
  --header 'Content-Type: application/json' \
  --data '{
  "total_shares": 5,
  "threshold": 2,
  "plugins_credentials": {
    "openbao-rs": {
      "vault_token": "xxxx"
    },
    "aws-rs": {
      "aws_access_key_id": "xxxx",
      "aws_secret_access_key": "xxxx"
    }
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "system-initialized",
  "message": "System initialized",
  "data": {
    "shards": ["xxx", "yyy", "zzz", "sss", "bbb"],
    "root_token": "pk_xxxx"
  }
}
```

`total_shares` and `threshold` configure how the unseal key is split: the key
is divided into `total_shares` shards, and `threshold` of them are required to
unlock the cluster. Record the returned `shards` and `root_token` securely;
they are not retrievable later.

### Unlock the nodes

Provide at least `threshold` shards.

```sh
curl -X POST 'http://127.0.0.1:21001/system/unlock' \
  --header 'Content-Type: application/json' \
  --data '{
  "shards": ["xxx", "yyy"]
}'
```

## Application setup

### Create an application

**Requires the root token** from system initialization.

```sh
curl -X POST 'http://127.0.0.1:21001/app/create' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer $ROOT_TOKEN' \
  --data '{
  "app_name": "my-testing-app"
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "app_id": "019e7d2e-9048-70d3-b910-e209bb21b21b",
    "app_name": "my-testing-app"
  }
}
```

### Create a principal

**Requires the root token.** Choose one of the identity methods below.

#### API key

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer $ROOT_TOKEN' \
  --data '{
  "name": "my-first-app-user",
  "kind": "human",
  "credential_kind": "apiKey"
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "pk_yyy",
    "principal_id": "019e7d30-493b-7263-acd4-a811db0a95df"
  }
}
```

The returned `token` is the principal's API key. Save it; it is used to
authenticate later.

#### TPM

TPM requires an extra activation step after creation. See
[TPM activation](#tpm-activation).

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer $ROOT_TOKEN' \
  --data '{
  "name": "my-first-app-user-13",
  "kind": "human",
  "credential_kind": "trustedPlatformModule",
  "data": {
    "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsWnTrYtkNp8TOWn0Q2Ey\nZgfaSEngOdH15oZbWZbW9vzz/BJReYmitdnj4bNiO4S5lfMOYBk1uImNtqyYZAFQ\nv2q7Fj6TKYSD4WfWGvoT79o+ONcows2BexOrF4iWXpmYU0uBTyXDjFfcd6vMq0lY\nWhmPq3lfzbVmb0+in4RsTv+wEBU479jejnXYXWak0DeuFD5mpx15phRLq7r66olR\n2qAXZFoiiIfKhIk8xriNrmHG4aTFcRyBycmnA9aY2NHTZ4DPUJRo98YEqVoZqiu1\na5PVcjiwK8ia0fap6WAP4GxiheLCbARw9O8/aDqIlp7Gq5AfRnsRIISxMHYF8Fr9\nrQIDAQAB\n-----END PUBLIC KEY-----",
    "ek_cert_pem": null,
    "ak_public_tpm2b_b64": "ARgAAQALAAUAcgAAABAAFAALCAAAAAAAAQCx/4iVc4/T66m1lpeRbGDsyI41IStgsk24noh4eCxmWkDyhc3/D2mWwZWwNHH/puASbgLaZhfVnSBPRUfwraHfI6paswfEkuXiC5EFjnEg9iBPVyDz4rRk4kxDonHmVg7BS6lX4Ck8eiY+O3fJHElaq5EhfNgM38lwdVour9ehPisEDmSMJk1bUPbOv2Ahg77Fcz58jPBKCl8n91H2D7wseVjzXqJLDWxfC7u5UTybZMzJuEBwXT4nxK3faqB2OFKAOt/YsagJN0Lr/RbxlcIekbhPafEwYuhWHDtDFVVGTHVhDDS4nQAC+vh5fkrjMpjDx8XMXkerksOz3b55OPF7",
    "expected_pcrs": {
      "0": "0000000000000000000000000000000000000000000000000000000000000000",
      "1": "0000000000000000000000000000000000000000000000000000000000000000",
      "2": "0000000000000000000000000000000000000000000000000000000000000000",
      "3": "0000000000000000000000000000000000000000000000000000000000000000",
      "4": "0000000000000000000000000000000000000000000000000000000000000000",
      "5": "0000000000000000000000000000000000000000000000000000000000000000",
      "6": "0000000000000000000000000000000000000000000000000000000000000000",
      "7": "0000000000000000000000000000000000000000000000000000000000000000"
    },
    "require_ek_cert": false
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "credential_blob": "AEQAIG9HXJb7EmBBThn4njBjTevYmx0bwd18hkxpH6qEiGsZw5CerC6CQK1iutFY0Pktf5ML/L2cnRmUerZNDpvU37WUQQ==",
    "secret": "AQBFHDgxet2FLu31vaLSQlJZoKginCNquIHfvU6hkcrU7Pz0hYk4pEqpnJvDRaSDUVPYc8d5Le5KGPZ0Njfc4NABKE/ZihNR9jdFlf5hGdWo4buavUrhXakrz6OhUBZrTtNzXVpjV9Qvqu0oAJEYcc94Xx/puRNXHq2JvqKHjr9tyKSv1fNWMvtWwFcgNBbeUtfmsK1mfGe2fDLpkWsC2ut0F1bvogjCN8fKSkGEtyE1zh1tuSvchJwOc3VjnIGD3yHnGWHnxwCvHF9pHerrdXmBsGYjpiooClngtC1q73JGV3357YG9MoqrTsW5v04zaT/bXifZllKxIMzLqfgqQOCo",
    "principal_id": "019e9270-bc44-7862-a8f6-cfcd351568c0",
    "credential_id": "019e9270-bc55-73e0-9e35-1c2b4f5d28d0"
  }
}
```

#### TPM activation

Activate the credential created above using the recovered secret. Substitute
your own `recovered_secret`.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth/activate' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "trustedPlatformModule",
  "principal_id": "$PRINCIPAL_ID",
  "credential_id": "$CREDENTIAL_ID",
  "proof": {
    "recovered_secret": "upkRgrr+HjToV6eOJ2SFnQkachQ++Wtrb0DuqTYlGtw="
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed"
}
```

#### AWS IAM role

Create the IAM role in AWS first, then register it. Substitute your own
`role_arn`.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer $ROOT_TOKEN' \
  --data '{
  "name": "my-first-app-user-11",
  "kind": "human",
  "credential_kind": "awsIdentity",
  "data": {
    "role_arn": "arn:aws:iam::123456789012:role/decodering-test-role"
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "principal_id": "019e922a-27ed-7943-9938-9b101a55f633",
    "credential_id": "019e922a-27ed-7943-9938-9b202f27da41"
  }
}
```

### Grant the principal access to the application

**Requires the root token.**

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/grant' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer $ROOT_TOKEN' \
  --data '{
  "principal_id": "$PRINCIPAL_ID",
  "apps": ["$APP_ID"]
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed"
}
```

## Authentication

Authenticating as a principal returns a short-term token used for the OSL
endpoints.

### Request an auth challenge

TPM authentication requires a nonce challenge first. (API key authentication
does not need this step.)

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth/challenge' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "trustedPlatformModule"
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "challenge_id": "019e9270-ca03-7923-af50-078e8a9e3e88",
    "nonce": "cbde9f13935b08a4b7bcaad237dc1e25d1084bc8d87866d609627dee9bab2e7f",
    "expires_at": 1780573625
  }
}
```

### Authenticate and obtain a short-term token

#### API key authentication

`key` is the principal's API key returned by `/app/user/create`.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "apiKey",
  "proof": {
    "key": "$USER_API_KEY"
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "tok_xxx",
    "expires_at": 1780220675
  }
}
```

#### TPM authentication

Use the `challenge_id` from [Request an auth challenge](#request-an-auth-challenge).
Substitute your own TPM data.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "trustedPlatformModule",
  "proof": {
    "challenge_id": "$CHALLENGE_ID",
    "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsWnTrYtkNp8TOWn0Q2Ey\nZgfaSEngOdH15oZbWZbW9vzz/BJReYmitdnj4bNiO4S5lfMOYBk1uImNtqyYZAFQ\nv2q7Fj6TKYSD4WfWGvoT79o+ONcows2BexOrF4iWXpmYU0uBTyXDjFfcd6vMq0lY\nWhmPq3lfzbVmb0+in4RsTv+wEBU479jejnXYXWak0DeuFD5mpx15phRLq7r66olR\n2qAXZFoiiIfKhIk8xriNrmHG4aTFcRyBycmnA9aY2NHTZ4DPUJRo98YEqVoZqiu1\na5PVcjiwK8ia0fap6WAP4GxiheLCbARw9O8/aDqIlp7Gq5AfRnsRIISxMHYF8Fr9\nrQIDAQAB\n-----END PUBLIC KEY-----",
    "ak_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsf+IlXOP0+uptZaXkWxg\n7MiONSErYLJNuJ6IeHgsZlpA8oXN/w9plsGVsDRx/6bgEm4C2mYX1Z0gT0VH8K2h\n3yOqWrMHxJLl4guRBY5xIPYgT1cg8+K0ZOJMQ6Jx5lYOwUupV+ApPHomPjt3yRxJ\nWquRIXzYDN/JcHVaLq/XoT4rBA5kjCZNW1D2zr9gIYO+xXM+fIzwSgpfJ/dR9g+8\nLHlY816iSw1sXwu7uVE8m2TMybhAcF0+J8St32qgdjhSgDrf2LGoCTdC6/0W8ZXC\nHpG4T2nxMGLoVhw7QxVVRkx1YQw0uJ0AAvr4eX5K4zKYw8fFzF5Hq5LDs92+eTjx\newIDAQAB\n-----END PUBLIC KEY-----",
    "quote": "/1RDR4AYACIAC675HSzTDpOXRQsJKaiw+J6wkg8jsarkuv89khik4ku6ACD+J9YHdbouDcS5lpyflvZM+4TiZgV5N0CHNToUy3DuxwAAAAAJNI3UAAAABAAAAAEBICQBJQASAAAAAAABAAsD/wAAACBTQeayZGl5pw5XZTAHofMQFpQh7JvdnxpWSPda3gBa8Q==",
    "signature": "ABQACwEApxXzGwQD7T3llS4DX5drRDs89Pa2DzVnnO0AXj1mbIlL/4VeTL4lF2rjf0IhSewjzfEnU1iowEszMFz+/v2hRY/3fiJLiV6bsDEow8F9PscpmezV67tkGToR/m7QVD/PHebq1mb+o7ef1eMUlAo+HStP8JNYbdRlVfpmR0VSyKkFSwrQ/m6cLtH6Zoo9qJIVS+jF/O0V31uksb22x1CxYADg7kTAJoGLNFOQ8NDbkaWDO5ZZLQ+FAH/HT+rIhDtm7PxpWnkA+NzSRcyipaJUCzXiqeAYUgqpS4v7+kJ3kAWibjnlzJ0qjWlYZg5Kp8kG23kdXuH2xDXglEtFU4PttQ==",
    "pcrs": {
      "0": "0000000000000000000000000000000000000000000000000000000000000000",
      "1": "0000000000000000000000000000000000000000000000000000000000000000",
      "2": "0000000000000000000000000000000000000000000000000000000000000000",
      "3": "0000000000000000000000000000000000000000000000000000000000000000",
      "4": "0000000000000000000000000000000000000000000000000000000000000000",
      "5": "0000000000000000000000000000000000000000000000000000000000000000",
      "6": "0000000000000000000000000000000000000000000000000000000000000000",
      "7": "0000000000000000000000000000000000000000000000000000000000000000"
    }
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "tok_xxx",
    "expires_at": 1780220675
  }
}
```

#### AWS IAM role authentication

Substitute your own signed `GetCallerIdentity` request.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "awsIdentity",
  "proof": {
    "body": "Action=GetCallerIdentity&Version=2011-06-15",
    "headers": {
      "authorization": "AWS4-HMAC-SHA256 Credential=yyy/20260604/us-east-1/sts/aws4_request, SignedHeaders=content-type;host;x-amz-date;x-amz-security-token, Signature=0373cee2c987f48995ab917daf7c4ba1677eecc45a1d21067e3e50003e2ca2d2",
      "content-type": "application/x-www-form-urlencoded",
      "host": "sts.amazonaws.com",
      "x-amz-date": "20260604T104118Z",
      "x-amz-security-token": "xxx"
    },
    "method": "POST",
    "url": "https://sts.amazonaws.com/"
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "tok_xxx",
    "expires_at": 1780220675
  }
}
```

## Working with secrets (OSL)

Use the short-term token from the previous step.

### Put a secret

This example targets the OpenBao backend. To use AWS Secrets Manager instead,
set `store.backend_ref` to `aws-rs`.

```sh
curl -X POST 'http://127.0.0.1:21001/osl/v1/secrets/put' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer $SHORT_TERM_TOKEN' \
  --data '{
  "app_id": "$APP_ID",
  "secret_name": "my-database-credentials",
  "store": {
    "backend_ref": "openbao-rs",
    "store_path": "production-test/my-database-credentials"
  },
  "data": {
    "username": "db_user-new",
    "password": "super_secret_password-new"
  },
  "options": {
    "create_only": false
  }
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "secret_name": "my-database-credentials",
    "provider_version_id": "11"
  }
}
```

### Get a secret

`version` accepts a specific provider version id, or `"0"` for the current
version.

```sh
curl -X POST 'http://127.0.0.1:21001/osl/v1/secrets/get' \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer $SHORT_TERM_TOKEN' \
  --data '{
  "app_id": "$APP_ID",
  "secret_name": "my-database-credentials",
  "version": "0"
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "password": "super_secret_password-new",
    "username": "db_user-new",
    "metadata": {
      "resolved_backend_ref": "openbao-rs",
      "provider_version_id": "11"
    }
  }
}
```
