class Account {
  #balance = 0;
  static #count = 0;
  #log = [];
  constructor(initial) {
    this.#balance = initial;
    Account.#count++;
  }
  #record(op) {
    this.#log.push(op);
  }
  get #summary() {
    return this.#log.join("|");
  }
  deposit(n) {
    this.#balance += n;
    this.#record("+" + n);
    return this;
  }
  get balance() {
    return this.#balance;
  }
  history() {
    return this.#summary;
  }
  static count() {
    return Account.#count;
  }
  static #secret() {
    return "s";
  }
  static reveal() {
    return Account.#secret();
  }
  static isAccount(o) {
    return #balance in o;
  }
}
const a = new Account(10);
a.deposit(5).deposit(1);
new Account(0);
console.log(a.balance, a.history(), Account.count(), Account.reveal());
console.log(Account.isAccount(a), Account.isAccount({}), Object.keys(a).length);
try {
  Account.prototype.deposit.call({}, 1);
} catch (e) {
  console.log(e.constructor.name);
}
