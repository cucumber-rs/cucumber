# Phase46 packaged SDK behavior acceptance

Do not execute these files during implementation. The adopted KBD contract
contains the entire phase-end packet: preparation, eleven accepted behaviors,
then their eleven negative controls. All are required; zero selected is failure.

Preparation builds actual N-API and PyO3 native libraries, packs a private npm
archive and wheel, installs clean outside-checkout consumers and compiles an
external Rust application. The BDD cases exercise those packages. TypeScript
6.0.3 is pinned as an official stable compiler release for the declaration
consumer; it is not a claim about all TypeScript versions. Original commands,
exits, package hashes, paths and case observations are retained in the evidence
folder. Native archives stay locally under ignored artifacts; no publication.

SDK46_EVIDENCE may select a fresh directory only for an authorized repair.
Preparation refuses existing evidence. Each run gets its own disposable root.
The root is removed only after both packets and their receipts are retained.
A failed packet retains its consumer root for diagnosis; cleanup must be recorded.
The runtime pointer is ignored and never a certification artifact on its own.
