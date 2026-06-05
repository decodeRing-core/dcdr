# Getting Started

[TOC]

## **Raft (3-node cluster)**

Assuming you followed the steps above to configure everything and have been able to start a node without problems, here's a complete example of a Raft cluster with 3 nodes and OpenBao vault and AWS Secrets Manager configured using the default plugins. We'll create an app and put and retrieve secrets in each backend using the following identity methods:

- Api Key
- vTPM
- AWS Role.

### Start all 3 nodes.

```sh
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
```

```sh
cargo run --bin decodering-server -- --id 2 --addr 127.0.0.1:21002
```

```sh
cargo run --bin decodering-server -- --id 3 --addr 127.0.0.1:21003
```

### Initialize cluster

```sh
curl -X POST 'http://127.0.0.1:21001/raft/init' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "raft_init": [],
}'
```

### Add nodes as learners

```sh
curl -X POST 'http://127.0.0.1:21001/raft/add-learner' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '[2, "127.0.0.1:21002"]'

curl -X POST 'http://127.0.0.1:21001/raft/add-learner' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '[3, "127.0.0.1:21003"]'
```

### Verify nodes have been added as learners

```sh
curl -X POST 'http://127.0.0.1:21001/raft/metrics' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*'
```

```json
{
  "osl_version": "1.0.0",
  "status": "raft-metrics",
  "message": "Raft node metrics",
  "data": {
    "running_state": {
      "Ok": null
    },
    "id": 1,
    "current_term": 1,
    "vote": {
      "leader_id": {
        "term": 1,
        "voted_for": 1
      },
      "committed": true
    },
    "last_log_index": 6,
    "committed": {
      "leader_id": 1,
      "index": 6
    },
    "last_applied": {
      "leader_id": 1,
      "index": 6
    },
    "snapshot": {
      "leader_id": 1,
      "index": 4
    },
    "purged": {
      "leader_id": 1,
      "index": 2
    },
    "state": "Leader",
    "current_leader": 1,
    "millis_since_quorum_ack": 0,
    "last_quorum_acked": 1780215757178426002,
    "membership_config": {
      "log_id": {
        "leader_id": 1,
        "index": 6
      },
      "membership": {
        "configs": [[1]],
        "nodes": {
          "1": {
            "addr": "127.0.0.1:21001"
          },
          "2": {
            "addr": "127.0.0.1:21002"
          },
          "3": {
            "addr": "127.0.0.1:21003"
          }
        }
      }
    },
    "heartbeat": {
      "1": 1780215757177625002,
      "2": 1780215757172533794,
      "3": 1780215757172533793
    },
    "replication": {
      "1": {
        "leader_id": 1,
        "index": 6
      },
      "2": {
        "leader_id": 1,
        "index": 6
      },
      "3": {
        "leader_id": 1,
        "index": 6
      }
    }
  }
}
```

### Upgrade learners to voters

See `Endpoints` section to see extra functionality of `change-membership`

```sh
curl -X POST 'http://127.0.0.1:21001/raft/change-membership' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
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
    "log_id": {
      "leader_id": 1,
      "index": 8
    },
    "data": "Noop",
    "membership": {
      "configs": [[1, 2, 3]],
      "nodes": {
        "1": {
          "addr": "127.0.0.1:21001"
        },
        "2": {
          "addr": "127.0.0.1:21002"
        },
        "3": {
          "addr": "127.0.0.1:21003"
        }
      }
    }
  }
}
```

### Initialize system

Create root user and pass plugin credentials. This can only be run once. See `/system/plugin/config` if you want to update the plugin credentials.

```sh
curl -X POST 'http://127.0.0.1:21001/system/init' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
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

### Unlock nodes

```sh
curl -X POST 'http://127.0.0.1:21001/system/unlock' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "shards": [
      "xxx",
      "yyy"
  ],
}'
```

### Create Application

**Requires root token** obtained when intializing the system.

```sh
curl -X POST 'http://127.0.0.1:21001/app/create' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "app_name": "my-testing-app",
}
' \
  --header 'Authorization: Bearer pk_xxx'
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

### API Key identity

#### Create Application User/Principal

**Requires root token** obtained when intializing the system.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "name": "my-first-app-user",
  "kind": "human",
  "credential_kind": "apiKey",
}
' \
  --header 'Authorization: Bearer pk_xxx'
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

#### Authenticate user/principal for application to obtain short term token to access OSL endpoints

`key` is your user's api key returned from `/app/user/create` (an admin needs to create this for you)

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "app_id": "019e7d30-493b-7263-acd4-a811db0a95df",
  "key": "pk_yyy",
}
'
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

### TPM identity

#### Create Application User/Principal

**Requires root token** obtained when intializing the system.

Please adjust the params based on your TPM system. The below is just an example.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "name": "my-first-app-user-2",
  "kind": "human",
  "credential_kind": "trustedPlatformModule",
  "tpm": {
    "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsWnTrYtkNp8TOWn0Q2Ey\nZgfaSEngOdH15oZbWZbW9vzz/BJReYmitdnj4bNiO4S5lfMOYBk1uImNtqyYZAFQ\nv2q7Fj6TKYSD4WfWGvoT79o+ONcows2BexOrF4iWXpmYU0uBTyXDjFfcd6vMq0lY\nWhmPq3lfzbVmb0+in4RsTv+wEBU479jejnXYXWak0DeuFD5mpx15phRLq7r66olR\n2qAXZFoiiIfKhIk8xriNrmHG4aTFcRyBycmnA9aY2NHTZ4DPUJRo98YEqVoZqiu1\na5PVcjiwK8ia0fap6WAP4GxiheLCbARw9O8/aDqIlp7Gq5AfRnsRIISxMHYF8Fr9\nrQIDAQAB\n-----END PUBLIC KEY-----",
    "ek_cert_pem": null,
    "ak_public_tpm2b_b64": "ARgAAQALAAUAcgAAABAAFAALCAAAAAAAAQCdeA4fcWA2LRD9IuUlXAXXgRon2eGaLOXff4srqPg5RHFYVoqBEcav+A4d85aUpU4anW8uKQQNYQwRGzazYQJjmDzkWAgQTxyYBleaEKOexS9HC06twZHCahCDKygNsWpDhpob3638xtmnWMJwCdCjfKQMr2zSRTGBhg+YlEAk/fAja1nTdO4sOS2AbGjq5kTAGdin6PzaanC98dX7TRGCqGCVSNTMqMZCi+QwzCmtXfgGx/g07MV0XfgmS+a3R5oZ3Si2GNWhZBQgfapjvlLW6x76eDkLtAQBjp6Y0MsccvF2SudfX7qNBMwr2bR89IQ6awJxBq3vOZD8/TAYw8Yv",
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
}
' \
  --header 'Authorization: Bearer pk_xxx'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "TPM key added",
    "principal_id": "019e876d-c912-72e2-be11-d270586bae99",
    "credential_id": "019e876d-c91e-7e62-ae82-059655533271",
    "tpm": {
      "credential_blob": "AEQAIGxwsaag77BNhgLVuJzpoDQTyzXScxGxe9/DG/NJdA0yA0M3JaPNqLhKxtEFghw9ZBTJEfZpQP+RXDZRgUpJAdRdvg==",
      "secret": "AQBI6UJ+Ct9CJLJ6bjyFMPZRQ4SVn9n1aWE3tSOpyU79KX78sDDyJWsXNUShUf9naBK5oSbJdbH2Pp82briqfRDWIghk4ylx8d/UjmjsfgKyYDAi1MNix0vkqNTuGFODl+g1EnI8VJdX6DhaJpr1soNnxOHNh2RwnG4ULQww6rE4ZF0+zV82N4t3XNjMC8TlbdLpJxZem1ulYUXs+VMwjI5DdQIssCGxJoGQ1rpPLgnmRTcpZdlRLFJNUOFZq/STJngI9abE98fwNP8Fo38OKJeSdYX7/hF04ntmavu0pospEOxzcAcecWWi8GcUvT1+g5xMDBekjQmNcadgy26QbcB/"
    }
  }
}
```

#### Activate TPM credential

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/tpm/activate' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "principal_id": "019e876d-c912-72e2-be11-d270586bae99",
  "credential_id": "019e876d-c91e-7e62-ae82-059655533271",
  "recovered_secret": "sQn71hYitxqBYX79cP0TkZr68qn/rrzAMPuYTEb9MSE="
}
'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed"
}
```

#### Create TPM Challenge

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/tpm/challenge' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
}
'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "challenge_id": "019e8783-78e0-7b23-9d94-855bd09cee65",
    "nonce": "94269fcef7e5cc3c3af36864ca2d0f0162d3d5954b2eb4e0b94d38729e72f242",
    "expires_at": 1780390300
  }
}
```

#### Authenticate user/principal for application to obtain short term token to access OSL endpoints

Please adjust the params based on your TPM system. The below is just an example.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth/tpm' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "challenge_id": "019e8783-78e0-7b23-9d94-855bd09cee65",
  "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsWnTrYtkNp8TOWn0Q2Ey\nZgfaSEngOdH15oZbWZbW9vzz/BJReYmitdnj4bNiO4S5lfMOYBk1uImNtqyYZAFQ\nv2q7Fj6TKYSD4WfWGvoT79o+ONcows2BexOrF4iWXpmYU0uBTyXDjFfcd6vMq0lY\nWhmPq3lfzbVmb0+in4RsTv+wEBU479jejnXYXWak0DeuFD5mpx15phRLq7r66olR\n2qAXZFoiiIfKhIk8xriNrmHG4aTFcRyBycmnA9aY2NHTZ4DPUJRo98YEqVoZqiu1\na5PVcjiwK8ia0fap6WAP4GxiheLCbARw9O8/aDqIlp7Gq5AfRnsRIISxMHYF8Fr9\nrQIDAQAB\n-----END PUBLIC KEY-----",
  "ak_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAnXgOH3FgNi0Q/SLlJVwF\n14EaJ9nhmizl33+LK6j4OURxWFaKgRHGr/gOHfOWlKVOGp1vLikEDWEMERs2s2EC\nY5g85FgIEE8cmAZXmhCjnsUvRwtOrcGRwmoQgysoDbFqQ4aaG9+t/MbZp1jCcAnQ\no3ykDK9s0kUxgYYPmJRAJP3wI2tZ03TuLDktgGxo6uZEwBnYp+j82mpwvfHV+00R\ngqhglUjUzKjGQovkMMwprV34Bsf4NOzFdF34Jkvmt0eaGd0othjVoWQUIH2qY75S\n1use+ng5C7QEAY6emNDLHHLxdkrnX1+6jQTMK9m0fPSEOmsCcQat7zmQ/P0wGMPG\nLwIDAQAB\n-----END PUBLIC KEY-----",
  "quote": "/1RDR4AYACIAC+1u5uKHqLisPyceqTssrm7vqtJ29hEpMxBRNC36ti8tACCUJp/O9+XMPDrzaGTKLQ8BYtPVlUsutOC5TThynnLyQgAAAAABFnlVAAAABAAAAAABICQBJQASAAAAAAABAAsD/wAAACBTQeayZGl5pw5XZTAHofMQFpQh7JvdnxpWSPda3gBa8Q==",
  "signature": "ABQACwEAVuW9osVpkOH+i9gud+tdzq9cg/LCmq8l/d+HcC3X78c3Cu7VMlWUs755D18tyUgSSWNKq8X7SmZw3xYA4RiYq3mKyQrJXKY9xulm1MizkfAMANdUP0eyazIvfYQwOKAh9KC3gfS+FmLHrIaYftXWrpxLiMwGIVvKUgvkANtJzgf0niES0g0gVY0SZRfPnpbBcO8tQgrG7eTrHc9S3PH+TrYonQag66zF3HsAhV6lhZiUc/K37bPOKRRfJu5u+Szk00zQ26jQTpDTjox3jqHxHjSk81QPJzi0vujiMmFEppAX5/8+2Z7MzsVSRQxG1Y+xrABgyVBJV1LM9U3MW5/2tg==",
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
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "tok_qNTJLC48rcu6djhNWjasqzHSU7RBsNOG",
    "expires_at": 1780393849
  }
}
```

### AWS Role identity

#### Create Application User/Principal

**Requires root token** obtained when intializing the system.

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "name": "my-first-app-user-11",
  "kind": "human",
  "credential_kind": "awsIdentity",
  "aws": {
    "role_arn": "arn:aws:iam::195430954655:role/decodering-test-role"
  }
}
' \
  --header 'Authorization: Bearer pk_xxx'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "AWS role added",
    "principal_id": "019e87ab-2675-7492-8e3d-c81bc2a17bd0",
    "credential_id": "019e87ab-2675-7492-8e3d-c82214a403b8"
  }
}
```

#### Authenticate user/principal for application to obtain short term token to access OSL endpoints

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth/aws' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "body": "Action=GetCallerIdentity&Version=2011-06-15",
  "headers": {
    "authorization": "AWS4-HMAC-SHA256 Credential=ASIAS3AEXW2PW2S6PP3W/20260602/us-east-1/sts/aws4_request, SignedHeaders=content-type;host;x-amz-date;x-amz-security-token, Signature=93d8c4de93030a1a27143843ax35bde9b5805f22859dd213067e134d609fa93f",
    "content-type": "application/x-www-form-urlencoded",
    "host": "sts.amazonaws.com",
    "x-amz-date": "20260602T092945Z",
    "x-amz-security-token": "xxxx"
  },
  "method": "POST",
  "url": "https://sts.amazonaws.com/"
}'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed",
  "data": {
    "token": "tok_0oKBE94QS7EKMYkodvTY3nSzG83Kb8QW",
    "expires_at": 1780396403
  }
}
```

### Grant application access to user/principal

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/grant' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "principal_id": "019e7d30-493b-7263-acd4-a811db0a95df",
  "apps": [
    "019e7d2e-9048-70d3-b910-e209bb21b21b",
  ]
}
' \
  --header 'Authorization: Bearer pk_xxx'
```

```json
{
  "osl_version": "1.0.0",
  "status": "operation-completed",
  "message": "Operation completed"
}
```

### OSL

#### Put secret into the vault. Example using OpenBao plugin

Use your short term token obtained after authentication.

```sh
curl -X POST 'http://127.0.0.1:21001/osl/v1/secrets/put' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "app_id": "019e7d30-493b-7263-acd4-a811db0a95df",
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
}' \
  --header 'Authorization: Bearer tok_xxxx'
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

#### Get secret from vault. Example using OpenBao plugin

```sh
curl -X POST 'http://127.0.0.1:21001/osl/v1/secrets/get' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "app_id": "019e7d30-493b-7263-acd4-a811db0a95df",
  "secret_name": "my-database-credentials",
  "version": "0"
}
' \
  --header 'Authorization: Bearer tok_xxx'
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
