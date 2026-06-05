# **Endpoints**

[TOC]

## **Raft**

### Init

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

### Add Learner

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

### Metrics

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

### Shutdown node

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

### Change Membership

Modify the cluster's membership. Add or remove nodes as needed.

```
POST http://HOST:PORT/raft/change-membership
```

<details>
<summary>Request Body</summary>

#### Upgrade learners to voters

```json
{
  "AddVoterIds": [1, 2]
}
```

#### Add voters along with their corresponding nodes

```json
{
  "AddVoters": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

#### Remove voters (downgrades them to learners)

```json
{
  "RemoveVoters": [1, 2]
}
```

#### Replace all voters. Every new voter's node must already be a learner

```json
{
  "ReplaceAllVoters": [4, 5, 6]
}
```

#### Add nodes to membership as learners. Does not replace existing nodes

```json
{
  "AddNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

#### Add or replace nodes in the membership config

```json
{
  "SetNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

#### Remove learner nodes from membership.

```json
{
  "RemoveNodes": [1, 2]
}
```

#### Replace all learner nodes with a new set

```json
{
  "ReplaceAllNodes": {
    "1": { "addr": "127.0.0.1:21001" },
    "2": { "addr": "127.0.0.1:21002" }
  }
}
```

#### Batch operations

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

## **System**

### Init

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

### Unlock

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

### Update plugin configs

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

### Status

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

## **Application**

**All endpoints below require a root token**

### Create application

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

### Create app user/principal

Create an application user (principal).

```
POST http://HOST:PORT/app/user/create
```

<details>
<summary>Request Body</summary>

#### ApiKey

```json
{
  "name": "my-first-app-user",
  "kind": "human",
  "credential_kind": "apiKey"
}
```

#### TPM (Trusted Platform Module)

```json
{
  "name": "my-first-app-user-tpm",
  "kind": "machine",
  "credential_kind": "trustedPlatformModule",
  "data": {
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

#### AWS Identity

```json
{
  "name": "my-first-app-user-aws",
  "kind": "service",
  "credential_kind": "awsIdentity",
  "data": {
    "role_arn": "arn:aws:iam::12345678:role/decodering-role"
  }
}
```

</details>

### Auth challenge

Generate a auth challenge

```
POST http://HOST:PORT/app/auth/challenge
```

<details>
<summary>Request Body</summary>

```json
{
  "credential_kind": "trustedPlatformModule"
}
```

</details>

### Auth activation

Generate a auth activation. Not every authentication method requires an activation.

```
POST http://HOST:PORT/app/auth/challenge
```

<details>
<summary>Request Body</summary>

```json
{
  "credential_kind": "trustedPlatformModule",
  "principal_id": "019e919a-22d5-7483-9151-5fffff346a15",
  "credential_id": "019e919a-22e3-7b53-8e7f-02155ff20a42",
  "proof": {
    "recovered_secret": "upkRgrr+HjToV6eOJ2SFnQkachQ++Wtrb0DuqTYlGtw="
  }
}
```

</details>

### Authenticate app user

Authenticate an application user. Returns a short-term token to interact with OSL endpoints.

```
POST http://HOST:PORT/app/user/auth
```

<details>
<summary>Request Body</summary>

```json
{
  "credential_kind": "apiKey",
  "proof": {
    "key": "pk_xxxx"
  }
}
```

```json
{
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
}
```

```json
{
  "credential_kind": "awsIdentity",
  "proof": {
    "body": "Action=GetCallerIdentity&Version=2011-06-15",
    "headers": {
      "authorization": "AWS4-HMAC-SHA256 Credential=xxx/20260604/us-east-1/sts/aws4_request, SignedHeaders=content-type;host;x-amz-date;x-amz-security-token, Signature=0373cee2c987f48995ab917daf7c4ba1677eecc45a1d21067e3e50003e2ca2d2",
      "content-type": "application/x-www-form-urlencoded",
      "host": "sts.amazonaws.com",
      "x-amz-date": "20260604T104118Z",
      "x-amz-security-token": "xxxx"
    },
    "method": "POST",
    "url": "https://sts.amazonaws.com/"
  }
}
```

</details>

### Grant application access to user/principal

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

### Revoke application access to user/principal

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

### List applications

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
