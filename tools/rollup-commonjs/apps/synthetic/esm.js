import log from "./log.js";
import shout, { extra } from "./reassign.js";
import * as props from "./props.js";
import cycle from "./cycle-a.js";
log("esm start");
log("shout " + shout("y") + " " + extra);
log("props " + Object.keys(props).sort().join(",") + " " + props.one);
log("cycle " + cycle.fromB);
