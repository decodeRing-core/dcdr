# **DecodeRing Rust Implementation**

[TOC]

## **Overview**

The project is organized into the following workspaces:

#### **decodering-cli**

A command-line tool that lets operators interact with decodering-server without calling the REST API directly.

#### **decodering-core**

Contains the abstractions shared across the codebase: plugins, actions, requests, responses, and other core logic. _This workspace must not depend on any other workspace in the project._

#### **decodering-db**

Concrete implementations of the storage backends. Currently supports SQLite and PostgreSQL. Depends on decodering-core.

#### **decodering-plugins**

Plugins maintained by the DecodeRing team that integrate with different vault backends.

#### **decodering-raft**

Concrete implementation of the Raft consensus protocol for DecodeRing, built on the openraft crate. Depends on decodering-core and decodering-db.

#### decodering-server

Implements the OSL (Open Secrets Language) REST API and handles Raft node management, system initialization, and ongoing operations. Depends on decodering-core, decodering-db, and decodering-raft.

## **Getting Started**

1. Install the latest version of Rust (https://rust-lang.org/tools/install/)
2. RocksDB and SQLite bindings require a system C toolchain and LLVM/Clang development libraries to build. On Alpine: `apk add build-base clang-dev clang-libs llvm-dev`. On `Debian/Ubuntu: apt install build-essential clang libclang-dev`.
3. Clone repository
4. Create .env file with configuration. Adjust as needed.

```shell
# Only `raft` is supported at this time.
STORAGE_MODE=raft
SERVER_LOG_OUTPUT=both
# For development only; use a persistent path in production.
SERVER_LOG_DIR="/tmp"
SERVER_LOG_PREFIX="decodering"
SERVER_LOG_MAX_FILES=0
# For development only; use a persistent path in production.
RAFT_LOG_DIR="/tmp"
RAFT_LOG_PREFIX="decodering"
TRACING_LEVEL=error,decodering=debug,extism=error,extism_pdk=error,tracing_actix_web=info
# The plugins folder must exist. In this example it lives inside the dcdr-rs folder.
PLUGIN_DIR="plugins"
TPM_TRUST_DIR="/tmp"
```

#### **Compiling plugins**

Run:

```shell
rustup target list --installed
```

Install the wasm targets if you haven't already. The `wasm32-unknown-unknown` target means the plugin is not tied to any specific operating system or CPU architecture, it will run anywhere a WASM runtime is available. The `wasm32-wasip1` target includes WASI support, enabling access to system interfaces like the filesystem and environment variables.

```shell
rustup target add wasm32-unknown-unknown wasm32-wasip1
```

From the decoding-plugins folder, navigate into each vault plugin folder you want and run:

```shell
./build.sh
```

This compiles the plugins to WebAssembly and copies them into a plugins folder inside dcdr-rs (the repository directory). If you set PLUGIN_DIRECTORY to a different path, compile the plugins manually, see `build.sh` for details.
If the build succeeds, you should see a `compiled/` folder containing the `.wasm` files for each plugin you built.
Each plugin requires a manifest file. Create a `manifests/` folder next to `compiled/` and add a `.yaml` file for each plugin.

```yaml
wasm:
  - path: "plugins/compiled/openbao-rs.wasm"
allowed_hosts:
  - "127.0.0.1"
config:
  type: "OpenBao"
  vault_addr: "http://127.0.0.1:8200"
  kv_mount: "Your openbao kv mount"
```

```yaml
wasm:
  - path: "plugins/compiled/aws-rs.wasm"
allowed_hosts:
  - "secretsmanager.ap-southeast-2.amazonaws.com"
config:
  type: "AWS Secrets Manager"
  region: "ap-southeast-2"
```

The final layout should look like:

```shell
dcdr-rs
  |- ...
  |- plugins
    |- compiled
      |- openbao-rs.wasm
      |- aws-rs.wasm
    |- manifests
      |- openbao-rs.yaml
      |- aws-rs.yaml
```

### **Run Nodes**

From the dcdr-rs directory, start a node with:

```sh
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
```

You can start additional nodes by incrementing the ID and port:

```sh
cargo run --bin decodering-server -- --id 2 --addr 127.0.0.1:21002
```

### Build Errors

**`Unable to find libclang: ... Dynamic loading not supported`**

On Alpine, Rust defaults to statically-linked musl binaries, and static musl binaries can't `dlopen` shared libraries. Since `bindgen` loads `libclang.so` dynamically at build time, the build fails.

Fix by disabling static CRT linking so binaries are dynamically linked:

```sh
export RUSTFLAGS="-C target-feature=-crt-static"
```

To make this persistent, add it to `~/.cargo/config.toml`:

```toml
[build]
rustflags = ["-C", "target-feature=-crt-static"]
```

### **Raft Example (3-node cluster)**

Assuming you followed the steps above to configure everything and have been able to start a node without problems, here's a complete example of a Raft cluster with 3 nodes and OpenBao vault and AWS Secrets Manager configured using the default plugins. We'll create an app and put and retrieve secrets in each backend using the following identity methods:

- Api Key
- vTPM
- AWS Role.

#### Start all 3 nodes.

```sh
cargo run --bin decodering-server -- --id 1 --addr 127.0.0.1:21001
```

```sh
cargo run --bin decodering-server -- --id 2 --addr 127.0.0.1:21002
```

```sh
cargo run --bin decodering-server -- --id 3 --addr 127.0.0.1:21003
```

#### Initialize cluster

```sh
curl -X POST 'http://127.0.0.1:21001/raft/init' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*' \
  --header 'Content-Type: application/json' \
  --data '{
  "raft_init": [],
}'
```

#### Add nodes as learners

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

#### Add nodes as learners

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

#### Verify nodes have been added as learners

```sh
curl -X POST 'http://127.0.0.1:21001/raft/metrics' \
  --header 'User-Agent: yaak' \
  --header 'Accept: */*'
```

You should see the following response:

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

#### Upgrade learners to voters

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

You should see the following response:

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

#### Initialize system

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

You should see the following response:

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

---

## **Endpoints**

### **Raft**

#### Init

Initialize a brand new Raft cluster.

```
POST http://HOST:PORT/raft/init
```

<details>
<summary>Request Body</summary>

```json
{
  "raft_init": []
}
```

</details>

#### Add Learner

Add a new learner to the cluster. The first parameter is the ID of the learner and the second is the IP address.

```
POST http://HOST:PORT/raft/add-learner
```

<details>
<summary>Request Body</summary>

```json
[2, "127.0.0.1:21002"]
```

</details>

#### Metrics

View Raft node metrics.

```
POST http://HOST:PORT/raft/metrics
```

<details>
<summary>Request Body</summary>

```json
[]
```

</details>

#### Shutdown node

Shutdown Raft node gracefully. Make sure to first remove node from membership.

```
POST http://HOST:PORT/raft/shutdown
```

<details>
<summary>Request Body</summary>

```json
[]
```

</details>

#### Change Membership

Modify the cluster's membership. Add or remove nodes as needed.

```
POST http://HOST:PORT/raft/change-membership
```

<details>
<summary>Request Body</summary>

##### Upgrade learners to voters

```json
{
  "AddVoterIds": [1, 2]
}
```

##### Add voters along with their corresponding nodes

```json
{
  "AddVoters": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Remove voters (downgrades them to learners)

```json
{
  "RemoveVoters": [1, 2]
}
```

##### Replace all voters. Every new voter's node must already be a learner

```json
{
  "ReplaceAllVoters": [4, 5, 6]
}
```

##### Add nodes to membership as learners. Does not replace existing nodes

```json
{
  "AddNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Add or replace nodes in the membership config

```json
{
  "SetNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Remove learner nodes from membership.

```json
{
  "RemoveNodes": [1, 2]
}
```

##### Replace all learner nodes with a new set

```json
{
  "ReplaceAllNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

##### Batch operations

```json
{
  "Batch": [
    {
      "AddNodes": {
        "4": { "addr": "127.0.0.1:21004" },
        "5": { "addr": "127.0.0.1:21005" }
      }
    },
    { "AddVoterIds": [4, 5] },
    { "RemoveVoters": [2] },
    { "RemoveNodes": [2] }
  ]
}
```

</details>

### **System**

#### Init

Initialize the system. Returns the Shamir key shards and the root user token. This endpoint can only be called once; subsequent calls will return an error.

```
POST http://HOST:PORT/system/init
```

<details>
<summary>Request Body</summary>

```json
{
  "total_shares": 5,
  "threshold": 2,
  "plugin_credentials": {}
}
```

</details>

#### Unlock

Unlock node with shards.

```
POST http://HOST:PORT/system/unlock
```

<details>
<summary>Request Body</summary>

```json
{
  "shards": ["xxxx", "xxxx"]
}
```

</details>

#### Update plugin configs

Update plugin configuration credentials

```
POST http://HOST:PORT/system/plugin/config
```

<details>
<summary>Request Body</summary>

```json
{
  "plugins_credentials": {
    "openbao-rs": {
      "vault_token": "xxx"
    }
  }
}
```

</details>

#### Status

View node status.

```
POST http://HOST:PORT/system/status
```

<details>
<summary>Request Body</summary>

```json
{}
```

</details>

### **Application**

**All endpoints below require a root token**

#### Create application

Create a new application.

```
POST http://HOST:PORT/app/create
```

<details>
<summary>Request Body</summary>

```json
{
  "app_name": "my-testing-app"
}
```

</details>

#### Create app user/principal

Create an application user (principal).

```
POST http://HOST:PORT/app/user/create
```

<details>
<summary>Request Body</summary>

##### ApiKey

```json
{
  "name": "my-first-app-user",
  "kind": "human",
  "credential_kind": "apiKey"
}
```

##### TPM (Trusted Platform Module)

```json
{
  "name": "my-first-app-user-tpm",
  "kind": "machine",
  "credential_kind": "trustedPlatformModule",
  "tpm": {
    "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAp7WfKeRHDJtmOJ4pik9C\nD0BRc9U5SrGJ0ZS5I0nzSOUEu7H0+ANwB0UXj0hDm5/WIWHCB1WrcXpM1xCLfcPj\n6BuFiOLaHuif2rfBPEO6okshclaw3auzoxeiZhy7cV3GNykWbx4nRHrg1/qctlXZ\n57oJ9LEWbw178xj8Mtd5u3L0i8d5e6CqDrIdbofoZ/40dBvUEvrCAQDJc71v3lAB\nAT42oUQUX1AIFTJQW1PAnhy0R8noMFB1RFxoxKRSM+o9uADco+tXRT9Fv6PNunhF\nfQWnWPdPIDDN8P1ayr3bMeEdIHEtSPwFlF/5/BHsfe+Zu4uIZoKF+eXzuyUfzj9I\nTwIDAQAB\n-----END PUBLIC KEY-----",
    "ek_cert_pem": null,
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
```

##### AWS Identity

```json
{
  "name": "my-first-app-user-aws",
  "kind": "service",
  "credential_kind": "awsIdentity",
  "aws": {
    "role_arn": "arn:aws:iam::12345678:role/decodering-role"
  }
}
```

</details>

#### TPM challenge

Generate a nonce for TPM attestation.

```
POST http://HOST:PORT/app/tpm/challenge
```

<details>
<summary>Request Body</summary>

```json
{}
```

</details>

#### Authenticate app user with API key

Authenticate an application user with API key. Returns a short-term token to interact with OSL endpoints.

```
POST http://HOST:PORT/app/user/auth
```

<details>
<summary>Request Body</summary>

```json
{
  "app_id": "019e0168-0178-71d1-aa88-853318e70b28",
  "key": "pk_xxxx"
}
```

</details>

#### Authenticate app user with TPM

Authenticate an application user with TPM. Returns a short-term token to interact with OSL endpoints.

```
POST http://HOST:PORT/app/user/auth/tpm
```

<details>
<summary>Request Body</summary>

```json
{
  "challenge_id": "019e3919-141c-7f12-a314-5a958873f7b8",
  "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAp7WfKeRHDJtmOJ4pik9C\nD0BRc9U5SrGJ0ZS5I0nzSOUEu7H0+ANwB0UXj0hDm5/WIWHCB1WrcXpM1xCLfcPj\n6BuFiOLaHuif2rfBPEO6okshflaw3auzoxeiZhy7cV3GNykWbx4nRHrg1/qctlXZ\n57oJ9LEWbw178xj8Mtd5u3L0i8d5e6CqDrIdbofoZ/40dBvUEvrCMQDJc71v3lAB\nAT42oUQUX1AIFTJQW1PAnhy0R8noMFB1RFxoxKRSM+o9uADco+tXRT9Fv6PNunhF\nfQWnWPdPIDDN8P1ayr3bMeEdIHEtSPwFlF/5/BHsfe+Zu4uIZoKF+eXzuyUfzj9I\nTwIDAQAB\n-----END PUBLIC KEY-----",
  "ak_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA5QV0yUJDIPFB2z9Or5Qh\notIce1mh6yYh99DiLw/BXDYnq0w1QdeqFovsrjtAcSkgjZYD+0AxS+GbMy9t0w96\ne8npIzGV5DY5xxfYNoNACax4Ac2fINYpBMAIZgPGywkQCRCqMQOPij3hPeEyjdT/\nfp+eUI8PL48NOlKAaSsNnzeja0Bf3smI8NVCjtV4PZh0DAG8udpdAwscJHPLR+Ao\n5fWexjAzg1Ww82b60PiPMfDG9euQelKq2uEEMaoDxr2H/SGGJgSuvKWPFLzkzue3\niuxVd7vy6b3Mc+Vb8FV7aP9KIr2fAGWtv4sjft4der/mFzQUNGXT9oi7m9WSdaZt\nMQIDAQAB\n-----END PUBLIC KEY-----",
  "quote": "/1RDR4AYACIAC78NNa5TAzYhuJtLMctsMlySU2lNKSHxrbe+gvSB5Y2kRCAcOs2aoZYYO/vA64ydjqwPApmrPda5isFcCVtswgAbvgAAAAAXQ8SKAAAABgAAAAABIBkQIwAWNjYAAAABAAsD/wAAACBTQeayZGl5pw5XZTAHofMQFpQh7JvdnxpWSPda3gBa8Q==",
  "signature": "ABQACwEAN7H3STugyT7nn3imuwNzzQSgUm65/4ewyIRv3XIhKrus9rk33asLkPZBBplWsNH5oCoAWLE3TiMi2zWnVBuC2vRVrAK5V9JBcwUZjHC+cFY5f6Qic5SYS9lZkYQZ+pxWRTWYt1C1w+7oGRwQ/v7vYsAHvfuNY6zWGEIRUqKAK6yLTM7Q6nMZaODK/3IX5dluRzmMrqwtnNkiLPuFDp8mnjMjpndiZoVF0fB94ThijZ07+H3W7mlgMI0Ta4KIolhh36PYHDGXhFr8ijLXa600s3rcy/JivO0A1/3rZ3KdLvS+YwXd7fk2MluXYOKmZbHREYAVreNZzXe7grAdwofraQ==",
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
```

</details>

#### Authenticate app user with AWS role

Authenticate an application user with AWS role. Returns a short-term token to interact with OSL endpoints.

```
POST http://HOST:PORT/app/user/auth/aws
```

<details>
<summary>Request Body</summary>

```json
{
  "body": "Action=GetCallerIdentity&Version=2011-06-15",
  "headers": {
    "authorization": "AWS4-HMAC-SHA256 Credential=xxxx/20260518/us-east-1/sts/aws4_request, SignedHeaders=content-type;host;x-amz-date;x-amz-security-token, Signature=xxx",
    "content-type": "application/x-www-form-urlencoded",
    "host": "sts.amazonaws.com",
    "x-amz-date": "20260518T072914Z",
    "x-amz-security-token": "xxxx"
  },
  "method": "POST",
  "url": "https://sts.amazonaws.com/"
}
```

</details>

#### Grant application access to user/principal

Grant a user/principal access to one or more applications.

```
POST http://HOST:PORT/app/user/grant
```

<details>
<summary>Request Body</summary>

```json
{
  "principal_id": "019e3918-e972-7810-a4ed-70c5ac822738",
  "apps": ["019e391c-1277-7102-8a06-865bceb0d46c"]
}
```

</details>

#### Revoke application access to user/principal

Revoke a user/principal's access to an application.

```
POST http://HOST:PORT/app/user/revoke
```

<details>
<summary>Request Body</summary>

```json
{
  "principal_id": "019e1a67-c4cf-75d1-bda0-89e02075d5da",
  "app_id": "019e1a64-c72b-76b2-9c8e-43ea281c293a"
}
```

</details>

#### List applications

List applications user/principal's has access to.

```
POST http://HOST:PORT/app/list
```

<details>
<summary>Request Body</summary>

```json
{
  "principal_id": "019e68de-f1b9-7f10-809e-7f34e96434f4"
}
```

</details>

### **OSL**

**All endpoints below require a root token or a short-term token**

View OSL spec at https://gitlab.intra.decodering.org/core/osl

Current implementation status

- [x] Get secret
- [x] Put secret
- [x] Destroy secret
- [x] Delete secret
- [x] Restore secret
- [x] List secrets
- [x] Taint secret
- [x] Is secret tainted
- [x] Untaint secret
- [x] Get capabilities
- [x] Secrets describe
- [ ] Secrets versions list
- [ ] Secret versions get
- [ ] Issue credential
- [ ] Renew credential
- [ ] Revoke credential
- [ ] Put rotation policy
- [ ] Rotate secret
- [ ] Put sync
- [ ] Run sync
- [ ] Get sync status
- [ ] List syncs
- [ ] Delete sync
- [x] List applications
- [x] List backends
