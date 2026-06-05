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

### Create Application User/Principal

**Requires root token** obtained when intializing the system.

#### Api Key

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

#### TPM

TPM requires a another step to activate the credential. [Go to TPM Activation](#tpm-activate)

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
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
    "credential_blob": "AEQAIG9HXJb7EmBBThn4njBjTevYmx0bwd18hkxpH6qEiGsZw5CerC6CQK1iutFY0Pktf5ML/L2cnRmUerZNDpvU37WUQQ==",
    "secret": "AQBFHDgxet2FLu31vaLSQlJZoKginCNquIHfvU6hkcrU7Pz0hYk4pEqpnJvDRaSDUVPYc8d5Le5KGPZ0Njfc4NABKE/ZihNR9jdFlf5hGdWo4buavUrhXakrz6OhUBZrTtNzXVpjV9Qvqu0oAJEYcc94Xx/puRNXHq2JvqKHjr9tyKSv1fNWMvtWwFcgNBbeUtfmsK1mfGe2fDLpkWsC2ut0F1bvogjCN8fKSkGEtyE1zh1tuSvchJwOc3VjnIGD3yHnGWHnxwCvHF9pHerrdXmBsGYjpiooClngtC1q73JGV3357YG9MoqrTsW5v04zaT/bXifZllKxIMzLqfgqQOCo",
    "principal_id": "019e9270-bc44-7862-a8f6-cfcd351568c0",
    "credential_id": "019e9270-bc55-73e0-9e35-1c2b4f5d28d0"
  }
}
```

#### TPM Activate 

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth/activate' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "trustedPlatformModule",
  "principal_id": "019e919a-22d5-7483-9151-5fffff346a15",
  "credential_id": "019e919a-22e3-7b53-8e7f-02155ff20a42",
  "proof": { 
      "recovered_secret": "upkRgrr+HjToV6eOJ2SFnQkachQ++Wtrb0DuqTYlGtw="
  }
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


#### AWS Role

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/create' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "name": "my-first-app-user-11",
  "kind": "human",
  "credential_kind": "awsIdentity",
  "data": {
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
    "principal_id": "019e922a-27ed-7943-9938-9b101a55f633",
    "credential_id": "019e922a-27ed-7943-9938-9b202f27da41"
  }
}
```

### Authenticate user/principal for application to obtain short term token to access OSL endpoints


#### Api Key

`key` is your user's api key returned from `/app/user/create` (an admin needs to create this for you)

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "apiKey",
  "proof": {
    "key": "pk_xxx",
  }
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

#### TPM

TPM authentication requires a nonce challenge to be verified. [Go to Auth Challenge](#auth-challenge)

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "trustedPlatformModule",
  "proof": {
  "challenge_id": "019e919f-2a58-7aa3-9846-3e0542d1e1fc",
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

### AWS Role

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
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

### Auth Challenge

```sh
curl -X POST 'http://127.0.0.1:21001/app/user/auth/challenge' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "credential_kind": "trustedPlatformModule",
}
'
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
