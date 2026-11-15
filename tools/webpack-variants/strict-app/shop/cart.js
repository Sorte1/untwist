export class Cart {
  constructor() {
    this.items = [];
  }

  add(name, price, quantity = 1) {
    this.items.push({ name, price, quantity });
    return this;
  }

  count() {
    return this.items.reduce((sum, item) => sum + item.quantity, 0);
  }

  total() {
    return this.items.reduce((sum, item) => sum + item.price * item.quantity, 0);
  }
}
