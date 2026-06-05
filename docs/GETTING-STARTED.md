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

todo!()

#### AWS Role

todo!()

### Authenticate user/principal for application to obtain short term token to access OSL endpoints

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
