# Security Policy

decodeRing handles secrets, so we take security seriously. This document
explains how to report vulnerabilities and what to expect when you do.

> [!IMPORTANT]
> decodeRing is currently in **alpha** and has **not** undergone a formal
> security audit. It is not intended to protect production secrets. Treat
> known alpha limitations (see below) as expected gaps, not vulnerabilities.

## Supported Versions

As a pre-1.0 project, only the most recent release receives security fixes.

| Version         | Supported |
| --------------- | :-------: |
| latest (`main`) |    ✅     |
| older releases  |    ❌     |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues,
discussions, or pull requests.**

Instead, report privately using **either**:

- **Email:** [security@decodering.org](mailto:security@decodering.org)

Please include as much of the following as you can:

- The affected version, release tag, or commit hash
- A description of the issue and its potential impact
- Steps to reproduce, or a proof of concept
- Any suggested remediation

## What to Expect

- **Acknowledgement:** within **5 business days** of your report.
- **Status updates:** at least every **7 days** until the issue is resolved.
- **Disclosure:** we follow coordinated disclosure. We will work with you on a
  timeline and will not publicly disclose details until a fix is available.
- **Credit:** we are happy to credit you in the release notes and advisory
  unless you prefer to remain anonymous.

## Scope

In scope: vulnerabilities in decodeRing's own code, meaning the server, CLI,
core, storage, raft, auth, and first-party plugins maintained in this
repository.

Out of scope:

- Known alpha limitations and unimplemented features (see the project's
  Implementation Status), and the absence of a formal security audit.
- Vulnerabilities in third-party vault backends (e.g. OpenBao, AWS Secrets
  Manager) or other upstream dependencies. Please report those to the
  relevant project instead.
- Issues requiring physical access, social engineering, or a compromised
  host/operator.

## Safe Harbor

We consider security research conducted in good faith and in accordance with
this policy to be authorized. We will not pursue or support legal action
against researchers who act in good faith, avoid privacy violations and service
disruption, and give us a reasonable opportunity to address an issue before
disclosing it publicly.
