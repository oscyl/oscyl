# RFCs

An RFC (request for comments) is a short design document for a change that
affects more than one part of Oscyl. [GOVERNANCE.md](https://github.com/oscyl/oscyl/blob/main/GOVERNANCE.md)
says when one is required: any change to the driver contract, any change
that touches more than one driver or what every driver must provide, and any
change to the native library format.

## Process

1. Copy `0000-template.md` to `0000-short-title.md` and fill it in.
2. Open a pull request that adds only the RFC. Discussion happens there.
3. When the pull request is opened, rename the file to use the pull request
   number, for example `0042-patch-kinds.md`.
4. A core maintainer accepts the RFC by merging it with its status set to
   **Accepted**, or closes the pull request with the reasons written down.
   Driver maintainers of affected drivers are asked to review first.
5. Implementation follows in separate pull requests that link the RFC. A
   contract change and the fixes in every driver land together, in one pull
   request, so `main` always passes conformance.

An accepted RFC that turns out to be wrong is changed by a new RFC, which
marks the old one **Superseded**.
