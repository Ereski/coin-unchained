Coin Unchained is a distributed cryptocoin platform based on proof-of-stake and a convergent concensus mechanism. Coin Unchained does not rely a blockchain, and instead, every actor has their own verifiable and immutable transaction log. Even by investing excessive amounts of computation power and time, malicious actors can only have a limited impact in the concensus of legitimate nodes, and only for a short time.

At its core, the network stores multiple immutable transaction logs that are created and extended by their owners. Every transaction must be verified by a minimum amount of other nodes in the network within a fixed time frame before the transaction can be considered *certified*, otherwise they simply time out. Verifications, or *vets*, is the core coin of the system. Every transaction sent to the network for certification costs vets, and verifying transactions from other actors yield vets.

Every actor has an associated trust value, which influences how much weight verifications from that actor has. Transactions must have a minimum verification weight to be considered *certified*. Data from untrusted actors is simply ignored. However, both vets and trust are calculated locally for every actor a node cares about. These are continuosly calculated from the (immutable) state of multiple transaction logs and cannot be easily manipuated. Computation of these properties is deterministic, so every legitimate node in the network will reach a uniform concensus about trust, vet amount, and the actual state of the network.


The only "coin" that is inherent to the system are *vets*. New transactions must be vetted by multiple nodes in a fixed time frame before they are considered *certified* by other nodes in the network. But nodes do not assume that certified transactions are inherently valid, and invalid transactions lead to penalties to all parties involved. Vets also have varying weights depending on which node emits them, with older nodes being more valuable than newer ones. In addition, no number of new nodes are able to fully certify any single transaction.

Transactions require a positive vet to even be considered for certification. That is, the more a node uses the network, the more it must contribute to it. Vets can be transferred between accounts, but their weight is capped by the value of the receiving account.

Hijacking the network requires both computational power and time to build up a pool of nodes that carry a high vet weight. It is possible to fool legitimate nodes by marking invalid transactions as certified. However, all transactions are continuosly being verified and reverified. Transactions that break the rules cause penalties to all parties involved, meaning hijacks are extremely risky, expensive in both computation and time, and under a high risk of discovery before any gains can be made from the attempt.

Coin Unchained runs over a distributed hash table (DHT) with a high degree of replication. Nodes can store the whole database or a big chunk of it, so the network is extremely resilient to catastrophic losses. Any piece of data is cryptographically signed by multiple parties, so tampering is virtually impossible. Every piece of data, including the creation of new accounts, is fully verifiable. The network does not rely on mining in the blockchain mining sense, and there is no need to reach or strengthen consensus by reinforcing a consistent view at the cost of useless work. Either a transaction is valid, or it isn't, and that can be verified within a fixed amount of time since the transaction was created.

# Components

## Distributed Hash Table

### Actors

### State Chains

### Pruning

## Claims and Transactions

*Claims* are small, signed data fragments. They contain a payload, some metadata, and a cryptographic signature. *Transactions* are a collection of claims that may be signed by different actors. Transactions' claims either all fail, or all succeed. The expected payload of each claim and what "success" means, are all defined by the specific application to which a transaction refers to.

A transaction itself is a claim containing a vector of claims and a timestamp. The order of claims in a transaction may or may not be important depending on the application. Transactions are registered by pushing them into the DHT under the following key: `(actor, application, sequence)`.

- `actor`: ID of the actor that signed this transaction
- `application`: ID of the application for which this transaction applies
- `sequence`: transaction number starting from 0

The DHT rejects pushes, without penalty, that:

- Refer to an unknown actor or application ID
- Refer to a revoked actor or application ID
- Refer to untrusted actor
- Contains at least one invalid signature
- Has a sequence number not equal to zero that is not the highest known sequence number for that `(actor, application)` pair, plus one
- Has a sequence number equal to zero if there is at least one other known transaction for that `(actor, application)` pair
- Has a timestamp too different from the current time
- Has a timestamp earlier than the last transaction
- Has a payload size above a certain threshold
- The application may also restrict which actors may push certain kinds of claims

After a push, transactions enter an adjudication period. Other nodes will verify that the claims in a transaction are valid according to the current state of that actor in that application. This verification happens randomly and on a voluntary basis, actors cannot use the network without performing this kind of work.

There are three possible outcomes for a verification:

- The verifier claims that this transaction is correct and abides by the rules of the application for this actor
- The verifier claims that this transaction violates the rules of the application for this actor, and thus the actor must be penalized
- The verifier does not claim anything, wasting their resources

Verification claims are attached to the original under `valid` and `invalid` keys. Neither `valid` not `invalid` claim require further verification. `invalid` claims are also limited and cannot be made by actors under a specific *trust*, but they can be made *at any time*.

After a set amount of time passed from when the transaction started, its status may be one of:

- *timed out*: the transaction has no `invalid` claims but also not enough `valid` claims. The transaction is disconsidered by the network
- *certified*: `valid` claims must be valued at a minimum amount across different trust tiers, and it must not have any `invalid` claims. Certified claims can be considered valid by certain applications, while others might prefer to revalidate the transaction themselves (possibly attaching an `invalid` claim)
- *contested*: the transaction has at least on `invalid` claim. Every node will verify the transaction and punish whomever is wrong. This action is deterministic and equal for all legitimate nodes, so they will reach the same local consensus with no need for a global (and contestable) claim of penalty. Malicious nodes will simply desynchronize with the mainnet instead of affecting it

**TODO: uncles that take resposibility for younger actors**

## Trust

Every node has a local trust and *vet* (V) cache for the nodes it cares about. Both are derived from the immutable state of the DHT under deterministic rules, so concensus is an emergent property of the network. Manipulating this concensus requires actions that are verifiably against the rules, causing a concensus that such an actor is untrustworthy and should be ignored and blocked off the network. Furthemore, actions that cause this kind of widespread verification requires a combination of activity in the network and age, making a denial of service prohibitely expensive to conduct and damage, limited.

This is not a web of trust. Each node has its own trust on other nodes, and that trust eventually converges on the same amount for multiple nodes. This local trust is never communicated to other nodes.

## Vets

## Penalties and Proof of Distrust

## Applications

**TODO: publising requires expensive proof of work**

# Case Studies

## The Double Spending Problem

Assume an actor `X` has a walled with $100, or equivalent, and it wants to spend more than $100. It can either try to spend more than it has in a single transaction, or it can start multiple transactions that collectively spend more than $100 available to them. The former is trivial to be detected. In the latter case, `X` must perform at least the following:

- Publish a valid transaction request with seqnumber `a`, spending <$100
- Publish zero or more interim transaction requests
- Publish a valid transaction request with seqnumber `b > a`, spending <$100 but a large enough value that this will be an invalid transaction if `a` is accepted

Then:

1. Transaction `b` will only be considered for certification if `a` and all interim transactions are certified and valid. But it is possible to maliciously certify `b`.
2. If `b` is not certified in time, it will be rejected.
3. Assuming `b` is considered for cetification by legitimate nodes, it will be rejected and a penalty will be issued to `X`
4. Assuming `b` is maliciously certified, the receiver can still validate the chain of transactions. This invalid transaction will be eventually picked by legitimate nodes and penalties will be issued, severely hampering the attacker's ability to maliciously certify invalid transactions in the future.
