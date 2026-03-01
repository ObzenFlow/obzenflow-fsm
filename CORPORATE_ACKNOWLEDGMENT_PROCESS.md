# Corporate Contribution Acknowledgment Process

_Created: 2026-02-20_

## Purpose

ObzenFlow uses the Developer Certificate of Origin (DCO), not a Contributor Licence Agreement (CLA). The DCO is lightweight by design and works well for the vast majority of contributions.

However, some contributors are employed by companies whose intellectual property policies may create ambiguity about ownership of work produced by their employees. This is common in the technology industry, particularly where employment agreements include broad IP assignment clauses that cover work done outside of office hours or on personal equipment.

When a maintainer determines that a contributor's employment relationship may create IP ambiguity for the project, the maintainer may request a corporate acknowledgment before the contribution can be merged. This is documented in CONTRIBUTING.md under "Contribution provenance."

This process exists to protect the project, its users, and the contributor. It is not a reflection on the contributor's intentions or the quality of their work.

## When this process applies

A maintainer may request corporate acknowledgment for any contribution at their sole discretion. The maintainer is not required to provide a reason beyond citing this policy. The policy itself is the justification.

This is a standard open source project safeguard. Many projects in the Apache, Eclipse, and CNCF ecosystems have similar provisions. It is not targeted at any specific company or contributor.

## What the contributor needs to do

Ask someone at your employer with authority over intellectual property matters (typically a VP of Engineering, CTO, General Counsel, or equivalent) to send an email to:

**governance@obzenflow.dev**

The email should contain the following, on a company email address:

---

**Subject:** Corporate Contribution Acknowledgment for ObzenFlow

**Body:**

I, [full name], [title] at [company name], confirm the following on behalf of [company name]:

1. We are aware that [contributor name] intends to contribute to the ObzenFlow open source project (https://github.com/obzenflow).

2. [Company name] does not claim intellectual property rights over the contribution(s) described below, and consents to their release under the project's open source licence terms (MIT OR Apache-2.0).

3. This acknowledgment covers: [brief description of the contribution, or "all contributions by [contributor name] to the ObzenFlow project during their employment at [company name]"].

Signed,
[full name]
[title]
[company name]
[date]

---

## What happens after the email is received

- The maintainer verifies the email came from a company domain and that the sender holds an appropriate role.
- The email is retained privately by the project maintainer as part of the contribution provenance record.
- The PR can proceed through normal review and merge.
- The contributor does not need to repeat this process for subsequent contributions unless their employment or the scope of the acknowledgment changes.

## What happens if the employer declines or does not respond

If the employer declines to provide acknowledgment, or if the contributor is unable or unwilling to request it, the maintainer cannot merge the contribution. This is not a judgement on the contributor. It is a risk management decision to protect the project and its users from potential IP disputes.

The contributor is welcome to:

- Resubmit the contribution after their employment situation changes.
- Open an issue describing the improvement so that a different contributor (without the IP ambiguity) can implement it.
- Participate in design discussions, issue triage, and other non-code contributions that do not raise IP concerns.

## Scope and intent

This process is a lightweight safeguard, not a barrier to contribution. The vast majority of contributions will never trigger it. It exists because the project takes intellectual property provenance seriously, and because the downstream users of ObzenFlow deserve confidence that every line of code in the project is cleanly licensed.

ObzenFlow values every contributor and does not use this process to discourage participation. If you have questions or concerns about this process, please reach out to the project maintainer directly.
