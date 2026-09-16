# Security Policy

Aiden can interact with serial hardware, local files, application launching, approved commands, keyboard input, and mouse input. Security reports involving these areas should be handled privately.

## Reporting a Vulnerability

Do **not** open a public GitHub issue for a suspected security vulnerability.

Report it privately to:

**awrynetwork@gmail.com**

Use a subject such as:

```text
Aiden Security Report
```

Include enough information to reproduce and understand the problem, such as:

- affected Aiden version
- operating system
- relevant component
- reproduction steps
- expected behavior
- observed behavior
- security impact
- proof-of-concept details, if appropriate

Do not send passwords, API keys, private keys, authentication tokens, or unrelated personal information.

Biosignal recordings can be personal data. Include a recording only when it is genuinely necessary to reproduce the issue and you intentionally consent to sharing that recording.

## Security-Sensitive Areas

Examples include:

- command or application execution escaping expected approval boundaries
- arbitrary file access
- path-validation bypasses
- unintended keyboard or mouse injection
- failure of emergency stop or held-input cleanup
- serial protocol parsing vulnerabilities
- unsafe import/export handling
- Tauri IPC or capability exposure
- persistence/database corruption that creates unsafe behavior
- privilege escalation
- code execution from untrusted workspace/profile data

Ordinary bugs should use the public bug-report template instead.

## Supported Version

Security fixes are primarily targeted at the current source version and current public release.

Older builds may not receive separate fixes.

## Disclosure

Please allow the issue to be investigated before publishing detailed exploit information.

Aiden is an independent open-source project, so no guaranteed response or remediation timeline is promised.

## Medical and Safety Note

Security reporting is not a channel for medical advice or interpretation of biosignal data.

Aiden is not a medical device.
