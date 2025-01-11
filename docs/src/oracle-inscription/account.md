## Account

There are three types of accounts in the cydonia: [User](#user), [Agent](#agent), [Service](#service).

Each account represents as an ed25519 public key in the network, used for the [Transaction](#transaction) and [State](#state).

### 1. User

Users are the ones that create the agents and use the services provided by the agents.

#### 1.1. Validator

Validators are the ones host the artificial intelligences and the related services, however, the hosted services are not required to be the same in each node, for more details, please refer to the [Oracle Inscriptions](/oracle-inscriptions/index.html) section.

### 2. Agent

Agents could be created by users after the validation of the validators, each validator could have different requirements for creating agents.

### 3. Service

As known as `Tools` for the agents, services are the ones that provide the services to the agents, they could also be used by the validator nodes to
handle their custom logic.
