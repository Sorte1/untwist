import greet from "./greet.js";
import { Cart } from "./shop/cart.js";
import { formatPrice } from "./shop/format.js";
import * as tax from "./shop/tax.js";

const cart = new Cart();
cart.add("apple", 0.5, 4);
cart.add("bread", 2.25, 1);
console.log(greet("shopper"));
console.log(`items ${cart.count()} subtotal ${formatPrice(cart.total())}`);
console.log(`with tax ${formatPrice(tax.withTax(cart.total()))} at ${tax.RATE * 100}%`);
