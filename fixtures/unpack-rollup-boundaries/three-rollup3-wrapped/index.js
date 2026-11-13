(function () {
  var hostConfig;
  var hostHook;
(function () {
	'use strict';

	const _lut = [ '00', '01', '02', '03', '04', '05', '06', '07', '08', '09', '0a', '0b', '0c', '0d', '0e', '0f', '10', '11', '12', '13', '14', '15', '16', '17', '18', '19', '1a', '1b', '1c', '1d', '1e', '1f', '20', '21', '22', '23', '24', '25', '26', '27', '28', '29', '2a', '2b', '2c', '2d', '2e', '2f', '30', '31', '32', '33', '34', '35', '36', '37', '38', '39', '3a', '3b', '3c', '3d', '3e', '3f', '40', '41', '42', '43', '44', '45', '46', '47', '48', '49', '4a', '4b', '4c', '4d', '4e', '4f', '50', '51', '52', '53', '54', '55', '56', '57', '58', '59', '5a', '5b', '5c', '5d', '5e', '5f', '60', '61', '62', '63', '64', '65', '66', '67', '68', '69', '6a', '6b', '6c', '6d', '6e', '6f', '70', '71', '72', '73', '74', '75', '76', '77', '78', '79', '7a', '7b', '7c', '7d', '7e', '7f', '80', '81', '82', '83', '84', '85', '86', '87', '88', '89', '8a', '8b', '8c', '8d', '8e', '8f', '90', '91', '92', '93', '94', '95', '96', '97', '98', '99', '9a', '9b', '9c', '9d', '9e', '9f', 'a0', 'a1', 'a2', 'a3', 'a4', 'a5', 'a6', 'a7', 'a8', 'a9', 'aa', 'ab', 'ac', 'ad', 'ae', 'af', 'b0', 'b1', 'b2', 'b3', 'b4', 'b5', 'b6', 'b7', 'b8', 'b9', 'ba', 'bb', 'bc', 'bd', 'be', 'bf', 'c0', 'c1', 'c2', 'c3', 'c4', 'c5', 'c6', 'c7', 'c8', 'c9', 'ca', 'cb', 'cc', 'cd', 'ce', 'cf', 'd0', 'd1', 'd2', 'd3', 'd4', 'd5', 'd6', 'd7', 'd8', 'd9', 'da', 'db', 'dc', 'dd', 'de', 'df', 'e0', 'e1', 'e2', 'e3', 'e4', 'e5', 'e6', 'e7', 'e8', 'e9', 'ea', 'eb', 'ec', 'ed', 'ee', 'ef', 'f0', 'f1', 'f2', 'f3', 'f4', 'f5', 'f6', 'f7', 'f8', 'f9', 'fa', 'fb', 'fc', 'fd', 'fe', 'ff' ];

	let _seed = 1234567;


	const DEG2RAD = Math.PI / 180;
	const RAD2DEG = 180 / Math.PI;

	/**
	 * Generate a [UUID]{@link https://en.wikipedia.org/wiki/Universally_unique_identifier}
	 * (universally unique identifier).
	 *
	 * @return {string} The UUID.
	 */
	function generateUUID() {

		// http://stackoverflow.com/questions/105034/how-to-create-a-guid-uuid-in-javascript/21963136#21963136

		const d0 = Math.random() * 0xffffffff | 0;
		const d1 = Math.random() * 0xffffffff | 0;
		const d2 = Math.random() * 0xffffffff | 0;
		const d3 = Math.random() * 0xffffffff | 0;
		const uuid = _lut[ d0 & 0xff ] + _lut[ d0 >> 8 & 0xff ] + _lut[ d0 >> 16 & 0xff ] + _lut[ d0 >> 24 & 0xff ] + '-' +
				_lut[ d1 & 0xff ] + _lut[ d1 >> 8 & 0xff ] + '-' + _lut[ d1 >> 16 & 0x0f | 0x40 ] + _lut[ d1 >> 24 & 0xff ] + '-' +
				_lut[ d2 & 0x3f | 0x80 ] + _lut[ d2 >> 8 & 0xff ] + '-' + _lut[ d2 >> 16 & 0xff ] + _lut[ d2 >> 24 & 0xff ] +
				_lut[ d3 & 0xff ] + _lut[ d3 >> 8 & 0xff ] + _lut[ d3 >> 16 & 0xff ] + _lut[ d3 >> 24 & 0xff ];

		// .toLowerCase() here flattens concatenated strings to save heap memory space.
		return uuid.toLowerCase();

	}

	/**
	 * Clamps the given value between min and max.
	 *
	 * @param {number} value - The value to clamp.
	 * @param {number} min - The min value.
	 * @param {number} max - The max value.
	 * @return {number} The clamped value.
	 */
	function clamp( value, min, max ) {

		return Math.max( min, Math.min( max, value ) );

	}

	/**
	 * Computes the Euclidean modulo of the given parameters that
	 * is `( ( n % m ) + m ) % m`.
	 *
	 * @param {number} n - The first parameter.
	 * @param {number} m - The second parameter.
	 * @return {number} The Euclidean modulo.
	 */
	function euclideanModulo( n, m ) {

		// https://en.wikipedia.org/wiki/Modulo_operation

		return ( ( n % m ) + m ) % m;

	}

	/**
	 * Performs a linear mapping from range `<a1, a2>` to range `<b1, b2>`
	 * for the given value.
	 *
	 * @param {number} x - The value to be mapped.
	 * @param {number} a1 - Minimum value for range A.
	 * @param {number} a2 - Maximum value for range A.
	 * @param {number} b1 - Minimum value for range B.
	 * @param {number} b2 - Maximum value for range B.
	 * @return {number} The mapped value.
	 */
	function mapLinear( x, a1, a2, b1, b2 ) {

		return b1 + ( x - a1 ) * ( b2 - b1 ) / ( a2 - a1 );

	}

	/**
	 * Returns the percentage in the closed interval `[0, 1]` of the given value
	 * between the start and end point.
	 *
	 * @param {number} x - The start point
	 * @param {number} y - The end point.
	 * @param {number} value - A value between start and end.
	 * @return {number} The interpolation factor.
	 */
	function inverseLerp( x, y, value ) {

		// https://www.gamedev.net/tutorials/programming/general-and-gameplay-programming/inverse-lerp-a-super-useful-yet-often-overlooked-function-r5230/

		if ( x !== y ) {

			return ( value - x ) / ( y - x );

		} else {

			return 0;

		}

	}

	/**
	 * Returns a value linearly interpolated from two known points based on the given interval -
	 * `t = 0` will return `x` and `t = 1` will return `y`.
	 *
	 * @param {number} x - The start point
	 * @param {number} y - The end point.
	 * @param {number} t - The interpolation factor in the closed interval `[0, 1]`.
	 * @return {number} The interpolated value.
	 */
	function lerp( x, y, t ) {

		return ( 1 - t ) * x + t * y;

	}

	/**
	 * Smoothly interpolate a number from `x` to `y` in  a spring-like manner using a delta
	 * time to maintain frame rate independent movement. For details, see
	 * [Frame rate independent damping using lerp]{@link http://www.rorydriscoll.com/2016/03/07/frame-rate-independent-damping-using-lerp/}.
	 *
	 * @param {number} x - The current point.
	 * @param {number} y - The target point.
	 * @param {number} lambda - A higher lambda value will make the movement more sudden,
	 * and a lower value will make the movement more gradual.
	 * @param {number} dt - Delta time in seconds.
	 * @return {number} The interpolated value.
	 */
	function damp( x, y, lambda, dt ) {

		return lerp( x, y, 1 - Math.exp( - lambda * dt ) );

	}

	/**
	 * Returns a value that alternates between `0` and the given `length` parameter.
	 *
	 * @param {number} x - The value to pingpong.
	 * @param {number} [length=1] - The positive value the function will pingpong to.
	 * @return {number} The alternated value.
	 */
	function pingpong( x, length = 1 ) {

		// https://www.desmos.com/calculator/vcsjnyz7x4

		return length - Math.abs( euclideanModulo( x, length * 2 ) - length );

	}

	/**
	 * Returns a value in the range `[0,1]` that represents the percentage that `x` has
	 * moved between `min` and `max`, but smoothed or slowed down the closer `x` is to
	 * the `min` and `max`.
	 *
	 * See [Smoothstep]{@link http://en.wikipedia.org/wiki/Smoothstep} for more details.
	 *
	 * @param {number} x - The value to evaluate based on its position between min and max.
	 * @param {number} min - The min value. Any x value below min will be `0`.
	 * @param {number} max - The max value. Any x value above max will be `1`.
	 * @return {number} The alternated value.
	 */
	function smoothstep( x, min, max ) {

		if ( x <= min ) return 0;
		if ( x >= max ) return 1;

		x = ( x - min ) / ( max - min );

		return x * x * ( 3 - 2 * x );

	}

	/**
	 * A [variation on smoothstep]{@link https://en.wikipedia.org/wiki/Smoothstep#Variations}
	 * that has zero 1st and 2nd order derivatives at x=0 and x=1.
	 *
	 * @param {number} x - The value to evaluate based on its position between min and max.
	 * @param {number} min - The min value. Any x value below min will be `0`.
	 * @param {number} max - The max value. Any x value above max will be `1`.
	 * @return {number} The alternated value.
	 */
	function smootherstep( x, min, max ) {

		if ( x <= min ) return 0;
		if ( x >= max ) return 1;

		x = ( x - min ) / ( max - min );

		return x * x * x * ( x * ( x * 6 - 15 ) + 10 );

	}

	/**
	 * Returns a random integer from `<low, high>` interval.
	 *
	 * @param {number} low - The lower value boundary.
	 * @param {number} high - The upper value boundary
	 * @return {number} A random integer.
	 */
	function randInt( low, high ) {

		return low + Math.floor( Math.random() * ( high - low + 1 ) );

	}

	/**
	 * Returns a random float from `<low, high>` interval.
	 *
	 * @param {number} low - The lower value boundary.
	 * @param {number} high - The upper value boundary
	 * @return {number} A random float.
	 */
	function randFloat( low, high ) {

		return low + Math.random() * ( high - low );

	}

	/**
	 * Returns a random integer from `<-range/2, range/2>` interval.
	 *
	 * @param {number} range - Defines the value range.
	 * @return {number} A random float.
	 */
	function randFloatSpread( range ) {

		return range * ( 0.5 - Math.random() );

	}

	/**
	 * Returns a deterministic pseudo-random float in the interval `[0, 1]`.
	 *
	 * @param {number} [s] - The integer seed.
	 * @return {number} A random float.
	 */
	function seededRandom( s ) {

		if ( s !== undefined ) _seed = s;

		// Mulberry32 generator

		let t = _seed += 0x6D2B79F5;

		t = Math.imul( t ^ t >>> 15, t | 1 );

		t ^= t + Math.imul( t ^ t >>> 7, t | 61 );

		return ( ( t ^ t >>> 14 ) >>> 0 ) / 4294967296;

	}

	/**
	 * Converts degrees to radians.
	 *
	 * @param {number} degrees - A value in degrees.
	 * @return {number} The converted value in radians.
	 */
	function degToRad( degrees ) {

		return degrees * DEG2RAD;

	}

	/**
	 * Converts radians to degrees.
	 *
	 * @param {number} radians - A value in radians.
	 * @return {number} The converted value in degrees.
	 */
	function radToDeg( radians ) {

		return radians * RAD2DEG;

	}

	/**
	 * Returns `true` if the given number is a power of two.
	 *
	 * @param {number} value - The value to check.
	 * @return {boolean} Whether the given number is a power of two or not.
	 */
	function isPowerOfTwo( value ) {

		return ( value & ( value - 1 ) ) === 0 && value !== 0;

	}

	/**
	 * Returns the smallest power of two that is greater than or equal to the given number.
	 *
	 * @param {number} value - The value to find a POT for.
	 * @return {number} The smallest power of two that is greater than or equal to the given number.
	 */
	function ceilPowerOfTwo( value ) {

		return Math.pow( 2, Math.ceil( Math.log( value ) / Math.LN2 ) );

	}

	/**
	 * Returns the largest power of two that is less than or equal to the given number.
	 *
	 * @param {number} value - The value to find a POT for.
	 * @return {number} The largest power of two that is less than or equal to the given number.
	 */
	function floorPowerOfTwo( value ) {

		return Math.pow( 2, Math.floor( Math.log( value ) / Math.LN2 ) );

	}

	/**
	 * Sets the given quaternion from the [Intrinsic Proper Euler Angles]{@link https://en.wikipedia.org/wiki/Euler_angles}
	 * defined by the given angles and order.
	 *
	 * Rotations are applied to the axes in the order specified by order:
	 * rotation by angle `a` is applied first, then by angle `b`, then by angle `c`.
	 *
	 * @param {Quaternion} q - The quaternion to set.
	 * @param {number} a - The rotation applied to the first axis, in radians.
	 * @param {number} b - The rotation applied to the second axis, in radians.
	 * @param {number} c - The rotation applied to the third axis, in radians.
	 * @param {('XYX'|'XZX'|'YXY'|'YZY'|'ZXZ'|'ZYZ')} order - A string specifying the axes order.
	 */
	function setQuaternionFromProperEuler( q, a, b, c, order ) {

		const cos = Math.cos;
		const sin = Math.sin;

		const c2 = cos( b / 2 );
		const s2 = sin( b / 2 );

		const c13 = cos( ( a + c ) / 2 );
		const s13 = sin( ( a + c ) / 2 );

		const c1_3 = cos( ( a - c ) / 2 );
		const s1_3 = sin( ( a - c ) / 2 );

		const c3_1 = cos( ( c - a ) / 2 );
		const s3_1 = sin( ( c - a ) / 2 );

		switch ( order ) {

			case 'XYX':
				q.set( c2 * s13, s2 * c1_3, s2 * s1_3, c2 * c13 );
				break;

			case 'YZY':
				q.set( s2 * s1_3, c2 * s13, s2 * c1_3, c2 * c13 );
				break;

			case 'ZXZ':
				q.set( s2 * c1_3, s2 * s1_3, c2 * s13, c2 * c13 );
				break;

			case 'XZX':
				q.set( c2 * s13, s2 * s3_1, s2 * c3_1, c2 * c13 );
				break;

			case 'YXY':
				q.set( s2 * c3_1, c2 * s13, s2 * s3_1, c2 * c13 );
				break;

			case 'ZYZ':
				q.set( s2 * s3_1, s2 * c3_1, c2 * s13, c2 * c13 );
				break;

			default:
				console.warn( 'THREE.MathUtils: .setQuaternionFromProperEuler() encountered an unknown order: ' + order );

		}

	}

	/**
	 * Denormalizes the given value according to the given typed array.
	 *
	 * @param {number} value - The value to denormalize.
	 * @param {TypedArray} array - The typed array that defines the data type of the value.
	 * @return {number} The denormalize (float) value in the range `[0,1]`.
	 */
	function denormalize( value, array ) {

		switch ( array.constructor ) {

			case Float32Array:

				return value;

			case Uint32Array:

				return value / 4294967295.0;

			case Uint16Array:

				return value / 65535.0;

			case Uint8Array:

				return value / 255.0;

			case Int32Array:

				return Math.max( value / 2147483647.0, - 1.0 );

			case Int16Array:

				return Math.max( value / 32767.0, - 1.0 );

			case Int8Array:

				return Math.max( value / 127.0, - 1.0 );

			default:

				throw new Error( 'Invalid component type.' );

		}

	}

	/**
	 * Normalizes the given value according to the given typed array.
	 *
	 * @param {number} value - The float value in the range `[0,1]` to normalize.
	 * @param {TypedArray} array - The typed array that defines the data type of the value.
	 * @return {number} The normalize value.
	 */
	function normalize( value, array ) {

		switch ( array.constructor ) {

			case Float32Array:

				return value;

			case Uint32Array:

				return Math.round( value * 4294967295.0 );

			case Uint16Array:

				return Math.round( value * 65535.0 );

			case Uint8Array:

				return Math.round( value * 255.0 );

			case Int32Array:

				return Math.round( value * 2147483647.0 );

			case Int16Array:

				return Math.round( value * 32767.0 );

			case Int8Array:

				return Math.round( value * 127.0 );

			default:

				throw new Error( 'Invalid component type.' );

		}

	}

	/**
	 * @class
	 * @classdesc A collection of math utility functions.
	 * @hideconstructor
	 */
	const MathUtils = {
		DEG2RAD: DEG2RAD,
		RAD2DEG: RAD2DEG,
		/**
		 * Generate a [UUID]{@link https://en.wikipedia.org/wiki/Universally_unique_identifier}
		 * (universally unique identifier).
		 *
		 * @static
		 * @method
		 * @return {string} The UUID.
		 */
		generateUUID: generateUUID,
		/**
		 * Clamps the given value between min and max.
		 *
		 * @static
		 * @method
		 * @param {number} value - The value to clamp.
		 * @param {number} min - The min value.
		 * @param {number} max - The max value.
		 * @return {number} The clamped value.
		 */
		clamp: clamp,
		/**
		 * Computes the Euclidean modulo of the given parameters that
		 * is `( ( n % m ) + m ) % m`.
		 *
		 * @static
		 * @method
		 * @param {number} n - The first parameter.
		 * @param {number} m - The second parameter.
		 * @return {number} The Euclidean modulo.
		 */
		euclideanModulo: euclideanModulo,
		/**
		 * Performs a linear mapping from range `<a1, a2>` to range `<b1, b2>`
		 * for the given value.
		 *
		 * @static
		 * @method
		 * @param {number} x - The value to be mapped.
		 * @param {number} a1 - Minimum value for range A.
		 * @param {number} a2 - Maximum value for range A.
		 * @param {number} b1 - Minimum value for range B.
		 * @param {number} b2 - Maximum value for range B.
		 * @return {number} The mapped value.
		 */
		mapLinear: mapLinear,
		/**
		 * Returns the percentage in the closed interval `[0, 1]` of the given value
		 * between the start and end point.
		 *
		 * @static
		 * @method
		 * @param {number} x - The start point
		 * @param {number} y - The end point.
		 * @param {number} value - A value between start and end.
		 * @return {number} The interpolation factor.
		 */
		inverseLerp: inverseLerp,
		/**
		 * Returns a value linearly interpolated from two known points based on the given interval -
		 * `t = 0` will return `x` and `t = 1` will return `y`.
		 *
		 * @static
		 * @method
		 * @param {number} x - The start point
		 * @param {number} y - The end point.
		 * @param {number} t - The interpolation factor in the closed interval `[0, 1]`.
		 * @return {number} The interpolated value.
		 */
		lerp: lerp,
		/**
		 * Smoothly interpolate a number from `x` to `y` in  a spring-like manner using a delta
		 * time to maintain frame rate independent movement. For details, see
		 * [Frame rate independent damping using lerp]{@link http://www.rorydriscoll.com/2016/03/07/frame-rate-independent-damping-using-lerp/}.
		 *
		 * @static
		 * @method
		 * @param {number} x - The current point.
		 * @param {number} y - The target point.
		 * @param {number} lambda - A higher lambda value will make the movement more sudden,
		 * and a lower value will make the movement more gradual.
		 * @param {number} dt - Delta time in seconds.
		 * @return {number} The interpolated value.
		 */
		damp: damp,
		/**
		 * Returns a value that alternates between `0` and the given `length` parameter.
		 *
		 * @static
		 * @method
		 * @param {number} x - The value to pingpong.
		 * @param {number} [length=1] - The positive value the function will pingpong to.
		 * @return {number} The alternated value.
		 */
		pingpong: pingpong,
		/**
		 * Returns a value in the range `[0,1]` that represents the percentage that `x` has
		 * moved between `min` and `max`, but smoothed or slowed down the closer `x` is to
		 * the `min` and `max`.
		 *
		 * See [Smoothstep]{@link http://en.wikipedia.org/wiki/Smoothstep} for more details.
		 *
		 * @static
		 * @method
		 * @param {number} x - The value to evaluate based on its position between min and max.
		 * @param {number} min - The min value. Any x value below min will be `0`.
		 * @param {number} max - The max value. Any x value above max will be `1`.
		 * @return {number} The alternated value.
		 */
		smoothstep: smoothstep,
		/**
		 * A [variation on smoothstep]{@link https://en.wikipedia.org/wiki/Smoothstep#Variations}
		 * that has zero 1st and 2nd order derivatives at x=0 and x=1.
		 *
		 * @static
		 * @method
		 * @param {number} x - The value to evaluate based on its position between min and max.
		 * @param {number} min - The min value. Any x value below min will be `0`.
		 * @param {number} max - The max value. Any x value above max will be `1`.
		 * @return {number} The alternated value.
		 */
		smootherstep: smootherstep,
		/**
		 * Returns a random integer from `<low, high>` interval.
		 *
		 * @static
		 * @method
		 * @param {number} low - The lower value boundary.
		 * @param {number} high - The upper value boundary
		 * @return {number} A random integer.
		 */
		randInt: randInt,
		/**
		 * Returns a random float from `<low, high>` interval.
		 *
		 * @static
		 * @method
		 * @param {number} low - The lower value boundary.
		 * @param {number} high - The upper value boundary
		 * @return {number} A random float.
		 */
		randFloat: randFloat,
		/**
		 * Returns a random integer from `<-range/2, range/2>` interval.
		 *
		 * @static
		 * @method
		 * @param {number} range - Defines the value range.
		 * @return {number} A random float.
		 */
		randFloatSpread: randFloatSpread,
		/**
		 * Returns a deterministic pseudo-random float in the interval `[0, 1]`.
		 *
		 * @static
		 * @method
		 * @param {number} [s] - The integer seed.
		 * @return {number} A random float.
		 */
		seededRandom: seededRandom,
		/**
		 * Converts degrees to radians.
		 *
		 * @static
		 * @method
		 * @param {number} degrees - A value in degrees.
		 * @return {number} The converted value in radians.
		 */
		degToRad: degToRad,
		/**
		 * Converts radians to degrees.
		 *
		 * @static
		 * @method
		 * @param {number} radians - A value in radians.
		 * @return {number} The converted value in degrees.
		 */
		radToDeg: radToDeg,
		/**
		 * Returns `true` if the given number is a power of two.
		 *
		 * @static
		 * @method
		 * @param {number} value - The value to check.
		 * @return {boolean} Whether the given number is a power of two or not.
		 */
		isPowerOfTwo: isPowerOfTwo,
		/**
		 * Returns the smallest power of two that is greater than or equal to the given number.
		 *
		 * @static
		 * @method
		 * @param {number} value - The value to find a POT for.
		 * @return {number} The smallest power of two that is greater than or equal to the given number.
		 */
		ceilPowerOfTwo: ceilPowerOfTwo,
		/**
		 * Returns the largest power of two that is less than or equal to the given number.
		 *
		 * @static
		 * @method
		 * @param {number} value - The value to find a POT for.
		 * @return {number} The largest power of two that is less than or equal to the given number.
		 */
		floorPowerOfTwo: floorPowerOfTwo,
		/**
		 * Sets the given quaternion from the [Intrinsic Proper Euler Angles]{@link https://en.wikipedia.org/wiki/Euler_angles}
		 * defined by the given angles and order.
		 *
		 * Rotations are applied to the axes in the order specified by order:
		 * rotation by angle `a` is applied first, then by angle `b`, then by angle `c`.
		 *
		 * @static
		 * @method
		 * @param {Quaternion} q - The quaternion to set.
		 * @param {number} a - The rotation applied to the first axis, in radians.
		 * @param {number} b - The rotation applied to the second axis, in radians.
		 * @param {number} c - The rotation applied to the third axis, in radians.
		 * @param {('XYX'|'XZX'|'YXY'|'YZY'|'ZXZ'|'ZYZ')} order - A string specifying the axes order.
		 */
		setQuaternionFromProperEuler: setQuaternionFromProperEuler,
		/**
		 * Normalizes the given value according to the given typed array.
		 *
		 * @static
		 * @method
		 * @param {number} value - The float value in the range `[0,1]` to normalize.
		 * @param {TypedArray} array - The typed array that defines the data type of the value.
		 * @return {number} The normalize value.
		 */
		normalize: normalize,
		/**
		 * Denormalizes the given value according to the given typed array.
		 *
		 * @static
		 * @method
		 * @param {number} value - The value to denormalize.
		 * @param {TypedArray} array - The typed array that defines the data type of the value.
		 * @return {number} The denormalize (float) value in the range `[0,1]`.
		 */
		denormalize: denormalize
	};

	/**
	 * Class for representing a Quaternion. Quaternions are used in three.js to represent rotations.
	 *
	 * Iterating through a vector instance will yield its components `(x, y, z, w)` in
	 * the corresponding order.
	 *
	 * Note that three.js expects Quaternions to be normalized.
	 * ```js
	 * const quaternion = new THREE.Quaternion();
	 * quaternion.setFromAxisAngle( new THREE.Vector3( 0, 1, 0 ), Math.PI / 2 );
	 *
	 * const vector = new THREE.Vector3( 1, 0, 0 );
	 * vector.applyQuaternion( quaternion );
	 * ```
	 */
	class Quaternion {

		/**
		 * Constructs a new quaternion.
		 *
		 * @param {number} [x=0] - The x value of this quaternion.
		 * @param {number} [y=0] - The y value of this quaternion.
		 * @param {number} [z=0] - The z value of this quaternion.
		 * @param {number} [w=1] - The w value of this quaternion.
		 */
		constructor( x = 0, y = 0, z = 0, w = 1 ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isQuaternion = true;

			this._x = x;
			this._y = y;
			this._z = z;
			this._w = w;

		}

		/**
		 * Interpolates between two quaternions via SLERP. This implementation assumes the
		 * quaternion data are managed  in flat arrays.
		 *
		 * @param {Array<number>} dst - The destination array.
		 * @param {number} dstOffset - An offset into the destination array.
		 * @param {Array<number>} src0 - The source array of the first quaternion.
		 * @param {number} srcOffset0 - An offset into the first source array.
		 * @param {Array<number>} src1 -  The source array of the second quaternion.
		 * @param {number} srcOffset1 - An offset into the second source array.
		 * @param {number} t - The interpolation factor in the range `[0,1]`.
		 * @see {@link Quaternion#slerp}
		 */
		static slerpFlat( dst, dstOffset, src0, srcOffset0, src1, srcOffset1, t ) {

			// fuzz-free, array-based Quaternion SLERP operation

			let x0 = src0[ srcOffset0 + 0 ],
				y0 = src0[ srcOffset0 + 1 ],
				z0 = src0[ srcOffset0 + 2 ],
				w0 = src0[ srcOffset0 + 3 ];

			const x1 = src1[ srcOffset1 + 0 ],
				y1 = src1[ srcOffset1 + 1 ],
				z1 = src1[ srcOffset1 + 2 ],
				w1 = src1[ srcOffset1 + 3 ];

			if ( t === 0 ) {

				dst[ dstOffset + 0 ] = x0;
				dst[ dstOffset + 1 ] = y0;
				dst[ dstOffset + 2 ] = z0;
				dst[ dstOffset + 3 ] = w0;
				return;

			}

			if ( t === 1 ) {

				dst[ dstOffset + 0 ] = x1;
				dst[ dstOffset + 1 ] = y1;
				dst[ dstOffset + 2 ] = z1;
				dst[ dstOffset + 3 ] = w1;
				return;

			}

			if ( w0 !== w1 || x0 !== x1 || y0 !== y1 || z0 !== z1 ) {

				let s = 1 - t;
				const cos = x0 * x1 + y0 * y1 + z0 * z1 + w0 * w1,
					dir = ( cos >= 0 ? 1 : - 1 ),
					sqrSin = 1 - cos * cos;

				// Skip the Slerp for tiny steps to avoid numeric problems:
				if ( sqrSin > Number.EPSILON ) {

					const sin = Math.sqrt( sqrSin ),
						len = Math.atan2( sin, cos * dir );

					s = Math.sin( s * len ) / sin;
					t = Math.sin( t * len ) / sin;

				}

				const tDir = t * dir;

				x0 = x0 * s + x1 * tDir;
				y0 = y0 * s + y1 * tDir;
				z0 = z0 * s + z1 * tDir;
				w0 = w0 * s + w1 * tDir;

				// Normalize in case we just did a lerp:
				if ( s === 1 - t ) {

					const f = 1 / Math.sqrt( x0 * x0 + y0 * y0 + z0 * z0 + w0 * w0 );

					x0 *= f;
					y0 *= f;
					z0 *= f;
					w0 *= f;

				}

			}

			dst[ dstOffset ] = x0;
			dst[ dstOffset + 1 ] = y0;
			dst[ dstOffset + 2 ] = z0;
			dst[ dstOffset + 3 ] = w0;

		}

		/**
		 * Multiplies two quaternions. This implementation assumes the quaternion data are managed
		 * in flat arrays.
		 *
		 * @param {Array<number>} dst - The destination array.
		 * @param {number} dstOffset - An offset into the destination array.
		 * @param {Array<number>} src0 - The source array of the first quaternion.
		 * @param {number} srcOffset0 - An offset into the first source array.
		 * @param {Array<number>} src1 -  The source array of the second quaternion.
		 * @param {number} srcOffset1 - An offset into the second source array.
		 * @return {Array<number>} The destination array.
		 * @see {@link Quaternion#multiplyQuaternions}.
		 */
		static multiplyQuaternionsFlat( dst, dstOffset, src0, srcOffset0, src1, srcOffset1 ) {

			const x0 = src0[ srcOffset0 ];
			const y0 = src0[ srcOffset0 + 1 ];
			const z0 = src0[ srcOffset0 + 2 ];
			const w0 = src0[ srcOffset0 + 3 ];

			const x1 = src1[ srcOffset1 ];
			const y1 = src1[ srcOffset1 + 1 ];
			const z1 = src1[ srcOffset1 + 2 ];
			const w1 = src1[ srcOffset1 + 3 ];

			dst[ dstOffset ] = x0 * w1 + w0 * x1 + y0 * z1 - z0 * y1;
			dst[ dstOffset + 1 ] = y0 * w1 + w0 * y1 + z0 * x1 - x0 * z1;
			dst[ dstOffset + 2 ] = z0 * w1 + w0 * z1 + x0 * y1 - y0 * x1;
			dst[ dstOffset + 3 ] = w0 * w1 - x0 * x1 - y0 * y1 - z0 * z1;

			return dst;

		}

		/**
		 * The x value of this quaternion.
		 *
		 * @type {number}
		 * @default 0
		 */
		get x() {

			return this._x;

		}

		set x( value ) {

			this._x = value;
			this._onChangeCallback();

		}

		/**
		 * The y value of this quaternion.
		 *
		 * @type {number}
		 * @default 0
		 */
		get y() {

			return this._y;

		}

		set y( value ) {

			this._y = value;
			this._onChangeCallback();

		}

		/**
		 * The z value of this quaternion.
		 *
		 * @type {number}
		 * @default 0
		 */
		get z() {

			return this._z;

		}

		set z( value ) {

			this._z = value;
			this._onChangeCallback();

		}

		/**
		 * The w value of this quaternion.
		 *
		 * @type {number}
		 * @default 1
		 */
		get w() {

			return this._w;

		}

		set w( value ) {

			this._w = value;
			this._onChangeCallback();

		}

		/**
		 * Sets the quaternion components.
		 *
		 * @param {number} x - The x value of this quaternion.
		 * @param {number} y - The y value of this quaternion.
		 * @param {number} z - The z value of this quaternion.
		 * @param {number} w - The w value of this quaternion.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		set( x, y, z, w ) {

			this._x = x;
			this._y = y;
			this._z = z;
			this._w = w;

			this._onChangeCallback();

			return this;

		}

		/**
		 * Returns a new quaternion with copied values from this instance.
		 *
		 * @return {Quaternion} A clone of this instance.
		 */
		clone() {

			return new this.constructor( this._x, this._y, this._z, this._w );

		}

		/**
		 * Copies the values of the given quaternion to this instance.
		 *
		 * @param {Quaternion} quaternion - The quaternion to copy.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		copy( quaternion ) {

			this._x = quaternion.x;
			this._y = quaternion.y;
			this._z = quaternion.z;
			this._w = quaternion.w;

			this._onChangeCallback();

			return this;

		}

		/**
		 * Sets this quaternion from the rotation specified by the given
		 * Euler angles.
		 *
		 * @param {Euler} euler - The Euler angles.
		 * @param {boolean} [update=true] - Whether the internal `onChange` callback should be executed or not.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		setFromEuler( euler, update = true ) {

			const x = euler._x, y = euler._y, z = euler._z, order = euler._order;

			// http://www.mathworks.com/matlabcentral/fileexchange/
			// 	20696-function-to-convert-between-dcm-euler-angles-quaternions-and-euler-vectors/
			//	content/SpinCalc.m

			const cos = Math.cos;
			const sin = Math.sin;

			const c1 = cos( x / 2 );
			const c2 = cos( y / 2 );
			const c3 = cos( z / 2 );

			const s1 = sin( x / 2 );
			const s2 = sin( y / 2 );
			const s3 = sin( z / 2 );

			switch ( order ) {

				case 'XYZ':
					this._x = s1 * c2 * c3 + c1 * s2 * s3;
					this._y = c1 * s2 * c3 - s1 * c2 * s3;
					this._z = c1 * c2 * s3 + s1 * s2 * c3;
					this._w = c1 * c2 * c3 - s1 * s2 * s3;
					break;

				case 'YXZ':
					this._x = s1 * c2 * c3 + c1 * s2 * s3;
					this._y = c1 * s2 * c3 - s1 * c2 * s3;
					this._z = c1 * c2 * s3 - s1 * s2 * c3;
					this._w = c1 * c2 * c3 + s1 * s2 * s3;
					break;

				case 'ZXY':
					this._x = s1 * c2 * c3 - c1 * s2 * s3;
					this._y = c1 * s2 * c3 + s1 * c2 * s3;
					this._z = c1 * c2 * s3 + s1 * s2 * c3;
					this._w = c1 * c2 * c3 - s1 * s2 * s3;
					break;

				case 'ZYX':
					this._x = s1 * c2 * c3 - c1 * s2 * s3;
					this._y = c1 * s2 * c3 + s1 * c2 * s3;
					this._z = c1 * c2 * s3 - s1 * s2 * c3;
					this._w = c1 * c2 * c3 + s1 * s2 * s3;
					break;

				case 'YZX':
					this._x = s1 * c2 * c3 + c1 * s2 * s3;
					this._y = c1 * s2 * c3 + s1 * c2 * s3;
					this._z = c1 * c2 * s3 - s1 * s2 * c3;
					this._w = c1 * c2 * c3 - s1 * s2 * s3;
					break;

				case 'XZY':
					this._x = s1 * c2 * c3 - c1 * s2 * s3;
					this._y = c1 * s2 * c3 - s1 * c2 * s3;
					this._z = c1 * c2 * s3 + s1 * s2 * c3;
					this._w = c1 * c2 * c3 + s1 * s2 * s3;
					break;

				default:
					console.warn( 'THREE.Quaternion: .setFromEuler() encountered an unknown order: ' + order );

			}

			if ( update === true ) this._onChangeCallback();

			return this;

		}

		/**
		 * Sets this quaternion from the given axis and angle.
		 *
		 * @param {Vector3} axis - The normalized axis.
		 * @param {number} angle - The angle in radians.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		setFromAxisAngle( axis, angle ) {

			// http://www.euclideanspace.com/maths/geometry/rotations/conversions/angleToQuaternion/index.htm

			const halfAngle = angle / 2, s = Math.sin( halfAngle );

			this._x = axis.x * s;
			this._y = axis.y * s;
			this._z = axis.z * s;
			this._w = Math.cos( halfAngle );

			this._onChangeCallback();

			return this;

		}

		/**
		 * Sets this quaternion from the given rotation matrix.
		 *
		 * @param {Matrix4} m - A 4x4 matrix of which the upper 3x3 of matrix is a pure rotation matrix (i.e. unscaled).
		 * @return {Quaternion} A reference to this quaternion.
		 */
		setFromRotationMatrix( m ) {

			// http://www.euclideanspace.com/maths/geometry/rotations/conversions/matrixToQuaternion/index.htm

			// assumes the upper 3x3 of m is a pure rotation matrix (i.e, unscaled)

			const te = m.elements,

				m11 = te[ 0 ], m12 = te[ 4 ], m13 = te[ 8 ],
				m21 = te[ 1 ], m22 = te[ 5 ], m23 = te[ 9 ],
				m31 = te[ 2 ], m32 = te[ 6 ], m33 = te[ 10 ],

				trace = m11 + m22 + m33;

			if ( trace > 0 ) {

				const s = 0.5 / Math.sqrt( trace + 1.0 );

				this._w = 0.25 / s;
				this._x = ( m32 - m23 ) * s;
				this._y = ( m13 - m31 ) * s;
				this._z = ( m21 - m12 ) * s;

			} else if ( m11 > m22 && m11 > m33 ) {

				const s = 2.0 * Math.sqrt( 1.0 + m11 - m22 - m33 );

				this._w = ( m32 - m23 ) / s;
				this._x = 0.25 * s;
				this._y = ( m12 + m21 ) / s;
				this._z = ( m13 + m31 ) / s;

			} else if ( m22 > m33 ) {

				const s = 2.0 * Math.sqrt( 1.0 + m22 - m11 - m33 );

				this._w = ( m13 - m31 ) / s;
				this._x = ( m12 + m21 ) / s;
				this._y = 0.25 * s;
				this._z = ( m23 + m32 ) / s;

			} else {

				const s = 2.0 * Math.sqrt( 1.0 + m33 - m11 - m22 );

				this._w = ( m21 - m12 ) / s;
				this._x = ( m13 + m31 ) / s;
				this._y = ( m23 + m32 ) / s;
				this._z = 0.25 * s;

			}

			this._onChangeCallback();

			return this;

		}

		/**
		 * Sets this quaternion to the rotation required to rotate the direction vector
		 * `vFrom` to the direction vector `vTo`.
		 *
		 * @param {Vector3} vFrom - The first (normalized) direction vector.
		 * @param {Vector3} vTo - The second (normalized) direction vector.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		setFromUnitVectors( vFrom, vTo ) {

			// assumes direction vectors vFrom and vTo are normalized

			let r = vFrom.dot( vTo ) + 1;

			if ( r < 1e-8 ) { // the epsilon value has been discussed in #31286

				// vFrom and vTo point in opposite directions

				r = 0;

				if ( Math.abs( vFrom.x ) > Math.abs( vFrom.z ) ) {

					this._x = - vFrom.y;
					this._y = vFrom.x;
					this._z = 0;
					this._w = r;

				} else {

					this._x = 0;
					this._y = - vFrom.z;
					this._z = vFrom.y;
					this._w = r;

				}

			} else {

				// crossVectors( vFrom, vTo ); // inlined to avoid cyclic dependency on Vector3

				this._x = vFrom.y * vTo.z - vFrom.z * vTo.y;
				this._y = vFrom.z * vTo.x - vFrom.x * vTo.z;
				this._z = vFrom.x * vTo.y - vFrom.y * vTo.x;
				this._w = r;

			}

			return this.normalize();

		}

		/**
		 * Returns the angle between this quaternion and the given one in radians.
		 *
		 * @param {Quaternion} q - The quaternion to compute the angle with.
		 * @return {number} The angle in radians.
		 */
		angleTo( q ) {

			return 2 * Math.acos( Math.abs( clamp( this.dot( q ), - 1, 1 ) ) );

		}

		/**
		 * Rotates this quaternion by a given angular step to the given quaternion.
		 * The method ensures that the final quaternion will not overshoot `q`.
		 *
		 * @param {Quaternion} q - The target quaternion.
		 * @param {number} step - The angular step in radians.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		rotateTowards( q, step ) {

			const angle = this.angleTo( q );

			if ( angle === 0 ) return this;

			const t = Math.min( 1, step / angle );

			this.slerp( q, t );

			return this;

		}

		/**
		 * Sets this quaternion to the identity quaternion; that is, to the
		 * quaternion that represents "no rotation".
		 *
		 * @return {Quaternion} A reference to this quaternion.
		 */
		identity() {

			return this.set( 0, 0, 0, 1 );

		}

		/**
		 * Inverts this quaternion via {@link Quaternion#conjugate}. The
		 * quaternion is assumed to have unit length.
		 *
		 * @return {Quaternion} A reference to this quaternion.
		 */
		invert() {

			return this.conjugate();

		}

		/**
		 * Returns the rotational conjugate of this quaternion. The conjugate of a
		 * quaternion represents the same rotation in the opposite direction about
		 * the rotational axis.
		 *
		 * @return {Quaternion} A reference to this quaternion.
		 */
		conjugate() {

			this._x *= - 1;
			this._y *= - 1;
			this._z *= - 1;

			this._onChangeCallback();

			return this;

		}

		/**
		 * Calculates the dot product of this quaternion and the given one.
		 *
		 * @param {Quaternion} v - The quaternion to compute the dot product with.
		 * @return {number} The result of the dot product.
		 */
		dot( v ) {

			return this._x * v._x + this._y * v._y + this._z * v._z + this._w * v._w;

		}

		/**
		 * Computes the squared Euclidean length (straight-line length) of this quaternion,
		 * considered as a 4 dimensional vector. This can be useful if you are comparing the
		 * lengths of two quaternions, as this is a slightly more efficient calculation than
		 * {@link Quaternion#length}.
		 *
		 * @return {number} The squared Euclidean length.
		 */
		lengthSq() {

			return this._x * this._x + this._y * this._y + this._z * this._z + this._w * this._w;

		}

		/**
		 * Computes the Euclidean length (straight-line length) of this quaternion,
		 * considered as a 4 dimensional vector.
		 *
		 * @return {number} The Euclidean length.
		 */
		length() {

			return Math.sqrt( this._x * this._x + this._y * this._y + this._z * this._z + this._w * this._w );

		}

		/**
		 * Normalizes this quaternion - that is, calculated the quaternion that performs
		 * the same rotation as this one, but has a length equal to `1`.
		 *
		 * @return {Quaternion} A reference to this quaternion.
		 */
		normalize() {

			let l = this.length();

			if ( l === 0 ) {

				this._x = 0;
				this._y = 0;
				this._z = 0;
				this._w = 1;

			} else {

				l = 1 / l;

				this._x = this._x * l;
				this._y = this._y * l;
				this._z = this._z * l;
				this._w = this._w * l;

			}

			this._onChangeCallback();

			return this;

		}

		/**
		 * Multiplies this quaternion by the given one.
		 *
		 * @param {Quaternion} q - The quaternion.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		multiply( q ) {

			return this.multiplyQuaternions( this, q );

		}

		/**
		 * Pre-multiplies this quaternion by the given one.
		 *
		 * @param {Quaternion} q - The quaternion.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		premultiply( q ) {

			return this.multiplyQuaternions( q, this );

		}

		/**
		 * Multiplies the given quaternions and stores the result in this instance.
		 *
		 * @param {Quaternion} a - The first quaternion.
		 * @param {Quaternion} b - The second quaternion.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		multiplyQuaternions( a, b ) {

			// from http://www.euclideanspace.com/maths/algebra/realNormedAlgebra/quaternions/code/index.htm

			const qax = a._x, qay = a._y, qaz = a._z, qaw = a._w;
			const qbx = b._x, qby = b._y, qbz = b._z, qbw = b._w;

			this._x = qax * qbw + qaw * qbx + qay * qbz - qaz * qby;
			this._y = qay * qbw + qaw * qby + qaz * qbx - qax * qbz;
			this._z = qaz * qbw + qaw * qbz + qax * qby - qay * qbx;
			this._w = qaw * qbw - qax * qbx - qay * qby - qaz * qbz;

			this._onChangeCallback();

			return this;

		}

		/**
		 * Performs a spherical linear interpolation between quaternions.
		 *
		 * @param {Quaternion} qb - The target quaternion.
		 * @param {number} t - The interpolation factor in the closed interval `[0, 1]`.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		slerp( qb, t ) {

			if ( t === 0 ) return this;
			if ( t === 1 ) return this.copy( qb );

			const x = this._x, y = this._y, z = this._z, w = this._w;

			// http://www.euclideanspace.com/maths/algebra/realNormedAlgebra/quaternions/slerp/

			let cosHalfTheta = w * qb._w + x * qb._x + y * qb._y + z * qb._z;

			if ( cosHalfTheta < 0 ) {

				this._w = - qb._w;
				this._x = - qb._x;
				this._y = - qb._y;
				this._z = - qb._z;

				cosHalfTheta = - cosHalfTheta;

			} else {

				this.copy( qb );

			}

			if ( cosHalfTheta >= 1.0 ) {

				this._w = w;
				this._x = x;
				this._y = y;
				this._z = z;

				return this;

			}

			const sqrSinHalfTheta = 1.0 - cosHalfTheta * cosHalfTheta;

			if ( sqrSinHalfTheta <= Number.EPSILON ) {

				const s = 1 - t;
				this._w = s * w + t * this._w;
				this._x = s * x + t * this._x;
				this._y = s * y + t * this._y;
				this._z = s * z + t * this._z;

				this.normalize(); // normalize calls _onChangeCallback()

				return this;

			}

			const sinHalfTheta = Math.sqrt( sqrSinHalfTheta );
			const halfTheta = Math.atan2( sinHalfTheta, cosHalfTheta );
			const ratioA = Math.sin( ( 1 - t ) * halfTheta ) / sinHalfTheta,
				ratioB = Math.sin( t * halfTheta ) / sinHalfTheta;

			this._w = ( w * ratioA + this._w * ratioB );
			this._x = ( x * ratioA + this._x * ratioB );
			this._y = ( y * ratioA + this._y * ratioB );
			this._z = ( z * ratioA + this._z * ratioB );

			this._onChangeCallback();

			return this;

		}

		/**
		 * Performs a spherical linear interpolation between the given quaternions
		 * and stores the result in this quaternion.
		 *
		 * @param {Quaternion} qa - The source quaternion.
		 * @param {Quaternion} qb - The target quaternion.
		 * @param {number} t - The interpolation factor in the closed interval `[0, 1]`.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		slerpQuaternions( qa, qb, t ) {

			return this.copy( qa ).slerp( qb, t );

		}

		/**
		 * Sets this quaternion to a uniformly random, normalized quaternion.
		 *
		 * @return {Quaternion} A reference to this quaternion.
		 */
		random() {

			// Ken Shoemake
			// Uniform random rotations
			// D. Kirk, editor, Graphics Gems III, pages 124-132. Academic Press, New York, 1992.

			const theta1 = 2 * Math.PI * Math.random();
			const theta2 = 2 * Math.PI * Math.random();

			const x0 = Math.random();
			const r1 = Math.sqrt( 1 - x0 );
			const r2 = Math.sqrt( x0 );

			return this.set(
				r1 * Math.sin( theta1 ),
				r1 * Math.cos( theta1 ),
				r2 * Math.sin( theta2 ),
				r2 * Math.cos( theta2 ),
			);

		}

		/**
		 * Returns `true` if this quaternion is equal with the given one.
		 *
		 * @param {Quaternion} quaternion - The quaternion to test for equality.
		 * @return {boolean} Whether this quaternion is equal with the given one.
		 */
		equals( quaternion ) {

			return ( quaternion._x === this._x ) && ( quaternion._y === this._y ) && ( quaternion._z === this._z ) && ( quaternion._w === this._w );

		}

		/**
		 * Sets this quaternion's components from the given array.
		 *
		 * @param {Array<number>} array - An array holding the quaternion component values.
		 * @param {number} [offset=0] - The offset into the array.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		fromArray( array, offset = 0 ) {

			this._x = array[ offset ];
			this._y = array[ offset + 1 ];
			this._z = array[ offset + 2 ];
			this._w = array[ offset + 3 ];

			this._onChangeCallback();

			return this;

		}

		/**
		 * Writes the components of this quaternion to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number>} [array=[]] - The target array holding the quaternion components.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number>} The quaternion components.
		 */
		toArray( array = [], offset = 0 ) {

			array[ offset ] = this._x;
			array[ offset + 1 ] = this._y;
			array[ offset + 2 ] = this._z;
			array[ offset + 3 ] = this._w;

			return array;

		}

		/**
		 * Sets the components of this quaternion from the given buffer attribute.
		 *
		 * @param {BufferAttribute} attribute - The buffer attribute holding quaternion data.
		 * @param {number} index - The index into the attribute.
		 * @return {Quaternion} A reference to this quaternion.
		 */
		fromBufferAttribute( attribute, index ) {

			this._x = attribute.getX( index );
			this._y = attribute.getY( index );
			this._z = attribute.getZ( index );
			this._w = attribute.getW( index );

			this._onChangeCallback();

			return this;

		}

		/**
		 * This methods defines the serialization result of this class. Returns the
		 * numerical elements of this quaternion in an array of format `[x, y, z, w]`.
		 *
		 * @return {Array<number>} The serialized quaternion.
		 */
		toJSON() {

			return this.toArray();

		}

		_onChange( callback ) {

			this._onChangeCallback = callback;

			return this;

		}

		_onChangeCallback() {}

		*[ Symbol.iterator ]() {

			yield this._x;
			yield this._y;
			yield this._z;
			yield this._w;

		}

	}

	/**
	 * Class representing a 3D vector. A 3D vector is an ordered triplet of numbers
	 * (labeled x, y and z), which can be used to represent a number of things, such as:
	 *
	 * - A point in 3D space.
	 * - A direction and length in 3D space. In three.js the length will
	 * always be the Euclidean distance(straight-line distance) from `(0, 0, 0)` to `(x, y, z)`
	 * and the direction is also measured from `(0, 0, 0)` towards `(x, y, z)`.
	 * - Any arbitrary ordered triplet of numbers.
	 *
	 * There are other things a 3D vector can be used to represent, such as
	 * momentum vectors and so on, however these are the most
	 * common uses in three.js.
	 *
	 * Iterating through a vector instance will yield its components `(x, y, z)` in
	 * the corresponding order.
	 * ```js
	 * const a = new THREE.Vector3( 0, 1, 0 );
	 *
	 * //no arguments; will be initialised to (0, 0, 0)
	 * const b = new THREE.Vector3( );
	 *
	 * const d = a.distanceTo( b );
	 * ```
	 */
	class Vector3 {

		/**
		 * Constructs a new 3D vector.
		 *
		 * @param {number} [x=0] - The x value of this vector.
		 * @param {number} [y=0] - The y value of this vector.
		 * @param {number} [z=0] - The z value of this vector.
		 */
		constructor( x = 0, y = 0, z = 0 ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			Vector3.prototype.isVector3 = true;

			/**
			 * The x value of this vector.
			 *
			 * @type {number}
			 */
			this.x = x;

			/**
			 * The y value of this vector.
			 *
			 * @type {number}
			 */
			this.y = y;

			/**
			 * The z value of this vector.
			 *
			 * @type {number}
			 */
			this.z = z;

		}

		/**
		 * Sets the vector components.
		 *
		 * @param {number} x - The value of the x component.
		 * @param {number} y - The value of the y component.
		 * @param {number} z - The value of the z component.
		 * @return {Vector3} A reference to this vector.
		 */
		set( x, y, z ) {

			if ( z === undefined ) z = this.z; // sprite.scale.set(x,y)

			this.x = x;
			this.y = y;
			this.z = z;

			return this;

		}

		/**
		 * Sets the vector components to the same value.
		 *
		 * @param {number} scalar - The value to set for all vector components.
		 * @return {Vector3} A reference to this vector.
		 */
		setScalar( scalar ) {

			this.x = scalar;
			this.y = scalar;
			this.z = scalar;

			return this;

		}

		/**
		 * Sets the vector's x component to the given value
		 *
		 * @param {number} x - The value to set.
		 * @return {Vector3} A reference to this vector.
		 */
		setX( x ) {

			this.x = x;

			return this;

		}

		/**
		 * Sets the vector's y component to the given value
		 *
		 * @param {number} y - The value to set.
		 * @return {Vector3} A reference to this vector.
		 */
		setY( y ) {

			this.y = y;

			return this;

		}

		/**
		 * Sets the vector's z component to the given value
		 *
		 * @param {number} z - The value to set.
		 * @return {Vector3} A reference to this vector.
		 */
		setZ( z ) {

			this.z = z;

			return this;

		}

		/**
		 * Allows to set a vector component with an index.
		 *
		 * @param {number} index - The component index. `0` equals to x, `1` equals to y, `2` equals to z.
		 * @param {number} value - The value to set.
		 * @return {Vector3} A reference to this vector.
		 */
		setComponent( index, value ) {

			switch ( index ) {

				case 0: this.x = value; break;
				case 1: this.y = value; break;
				case 2: this.z = value; break;
				default: throw new Error( 'index is out of range: ' + index );

			}

			return this;

		}

		/**
		 * Returns the value of the vector component which matches the given index.
		 *
		 * @param {number} index - The component index. `0` equals to x, `1` equals to y, `2` equals to z.
		 * @return {number} A vector component value.
		 */
		getComponent( index ) {

			switch ( index ) {

				case 0: return this.x;
				case 1: return this.y;
				case 2: return this.z;
				default: throw new Error( 'index is out of range: ' + index );

			}

		}

		/**
		 * Returns a new vector with copied values from this instance.
		 *
		 * @return {Vector3} A clone of this instance.
		 */
		clone() {

			return new this.constructor( this.x, this.y, this.z );

		}

		/**
		 * Copies the values of the given vector to this instance.
		 *
		 * @param {Vector3} v - The vector to copy.
		 * @return {Vector3} A reference to this vector.
		 */
		copy( v ) {

			this.x = v.x;
			this.y = v.y;
			this.z = v.z;

			return this;

		}

		/**
		 * Adds the given vector to this instance.
		 *
		 * @param {Vector3} v - The vector to add.
		 * @return {Vector3} A reference to this vector.
		 */
		add( v ) {

			this.x += v.x;
			this.y += v.y;
			this.z += v.z;

			return this;

		}

		/**
		 * Adds the given scalar value to all components of this instance.
		 *
		 * @param {number} s - The scalar to add.
		 * @return {Vector3} A reference to this vector.
		 */
		addScalar( s ) {

			this.x += s;
			this.y += s;
			this.z += s;

			return this;

		}

		/**
		 * Adds the given vectors and stores the result in this instance.
		 *
		 * @param {Vector3} a - The first vector.
		 * @param {Vector3} b - The second vector.
		 * @return {Vector3} A reference to this vector.
		 */
		addVectors( a, b ) {

			this.x = a.x + b.x;
			this.y = a.y + b.y;
			this.z = a.z + b.z;

			return this;

		}

		/**
		 * Adds the given vector scaled by the given factor to this instance.
		 *
		 * @param {Vector3|Vector4} v - The vector.
		 * @param {number} s - The factor that scales `v`.
		 * @return {Vector3} A reference to this vector.
		 */
		addScaledVector( v, s ) {

			this.x += v.x * s;
			this.y += v.y * s;
			this.z += v.z * s;

			return this;

		}

		/**
		 * Subtracts the given vector from this instance.
		 *
		 * @param {Vector3} v - The vector to subtract.
		 * @return {Vector3} A reference to this vector.
		 */
		sub( v ) {

			this.x -= v.x;
			this.y -= v.y;
			this.z -= v.z;

			return this;

		}

		/**
		 * Subtracts the given scalar value from all components of this instance.
		 *
		 * @param {number} s - The scalar to subtract.
		 * @return {Vector3} A reference to this vector.
		 */
		subScalar( s ) {

			this.x -= s;
			this.y -= s;
			this.z -= s;

			return this;

		}

		/**
		 * Subtracts the given vectors and stores the result in this instance.
		 *
		 * @param {Vector3} a - The first vector.
		 * @param {Vector3} b - The second vector.
		 * @return {Vector3} A reference to this vector.
		 */
		subVectors( a, b ) {

			this.x = a.x - b.x;
			this.y = a.y - b.y;
			this.z = a.z - b.z;

			return this;

		}

		/**
		 * Multiplies the given vector with this instance.
		 *
		 * @param {Vector3} v - The vector to multiply.
		 * @return {Vector3} A reference to this vector.
		 */
		multiply( v ) {

			this.x *= v.x;
			this.y *= v.y;
			this.z *= v.z;

			return this;

		}

		/**
		 * Multiplies the given scalar value with all components of this instance.
		 *
		 * @param {number} scalar - The scalar to multiply.
		 * @return {Vector3} A reference to this vector.
		 */
		multiplyScalar( scalar ) {

			this.x *= scalar;
			this.y *= scalar;
			this.z *= scalar;

			return this;

		}

		/**
		 * Multiplies the given vectors and stores the result in this instance.
		 *
		 * @param {Vector3} a - The first vector.
		 * @param {Vector3} b - The second vector.
		 * @return {Vector3} A reference to this vector.
		 */
		multiplyVectors( a, b ) {

			this.x = a.x * b.x;
			this.y = a.y * b.y;
			this.z = a.z * b.z;

			return this;

		}

		/**
		 * Applies the given Euler rotation to this vector.
		 *
		 * @param {Euler} euler - The Euler angles.
		 * @return {Vector3} A reference to this vector.
		 */
		applyEuler( euler ) {

			return this.applyQuaternion( _quaternion$2.setFromEuler( euler ) );

		}

		/**
		 * Applies a rotation specified by an axis and an angle to this vector.
		 *
		 * @param {Vector3} axis - A normalized vector representing the rotation axis.
		 * @param {number} angle - The angle in radians.
		 * @return {Vector3} A reference to this vector.
		 */
		applyAxisAngle( axis, angle ) {

			return this.applyQuaternion( _quaternion$2.setFromAxisAngle( axis, angle ) );

		}

		/**
		 * Multiplies this vector with the given 3x3 matrix.
		 *
		 * @param {Matrix3} m - The 3x3 matrix.
		 * @return {Vector3} A reference to this vector.
		 */
		applyMatrix3( m ) {

			const x = this.x, y = this.y, z = this.z;
			const e = m.elements;

			this.x = e[ 0 ] * x + e[ 3 ] * y + e[ 6 ] * z;
			this.y = e[ 1 ] * x + e[ 4 ] * y + e[ 7 ] * z;
			this.z = e[ 2 ] * x + e[ 5 ] * y + e[ 8 ] * z;

			return this;

		}

		/**
		 * Multiplies this vector by the given normal matrix and normalizes
		 * the result.
		 *
		 * @param {Matrix3} m - The normal matrix.
		 * @return {Vector3} A reference to this vector.
		 */
		applyNormalMatrix( m ) {

			return this.applyMatrix3( m ).normalize();

		}

		/**
		 * Multiplies this vector (with an implicit 1 in the 4th dimension) by m, and
		 * divides by perspective.
		 *
		 * @param {Matrix4} m - The matrix to apply.
		 * @return {Vector3} A reference to this vector.
		 */
		applyMatrix4( m ) {

			const x = this.x, y = this.y, z = this.z;
			const e = m.elements;

			const w = 1 / ( e[ 3 ] * x + e[ 7 ] * y + e[ 11 ] * z + e[ 15 ] );

			this.x = ( e[ 0 ] * x + e[ 4 ] * y + e[ 8 ] * z + e[ 12 ] ) * w;
			this.y = ( e[ 1 ] * x + e[ 5 ] * y + e[ 9 ] * z + e[ 13 ] ) * w;
			this.z = ( e[ 2 ] * x + e[ 6 ] * y + e[ 10 ] * z + e[ 14 ] ) * w;

			return this;

		}

		/**
		 * Applies the given Quaternion to this vector.
		 *
		 * @param {Quaternion} q - The Quaternion.
		 * @return {Vector3} A reference to this vector.
		 */
		applyQuaternion( q ) {

			// quaternion q is assumed to have unit length

			const vx = this.x, vy = this.y, vz = this.z;
			const qx = q.x, qy = q.y, qz = q.z, qw = q.w;

			// t = 2 * cross( q.xyz, v );
			const tx = 2 * ( qy * vz - qz * vy );
			const ty = 2 * ( qz * vx - qx * vz );
			const tz = 2 * ( qx * vy - qy * vx );

			// v + q.w * t + cross( q.xyz, t );
			this.x = vx + qw * tx + qy * tz - qz * ty;
			this.y = vy + qw * ty + qz * tx - qx * tz;
			this.z = vz + qw * tz + qx * ty - qy * tx;

			return this;

		}

		/**
		 * Projects this vector from world space into the camera's normalized
		 * device coordinate (NDC) space.
		 *
		 * @param {Camera} camera - The camera.
		 * @return {Vector3} A reference to this vector.
		 */
		project( camera ) {

			return this.applyMatrix4( camera.matrixWorldInverse ).applyMatrix4( camera.projectionMatrix );

		}

		/**
		 * Unprojects this vector from the camera's normalized device coordinate (NDC)
		 * space into world space.
		 *
		 * @param {Camera} camera - The camera.
		 * @return {Vector3} A reference to this vector.
		 */
		unproject( camera ) {

			return this.applyMatrix4( camera.projectionMatrixInverse ).applyMatrix4( camera.matrixWorld );

		}

		/**
		 * Transforms the direction of this vector by a matrix (the upper left 3 x 3
		 * subset of the given 4x4 matrix and then normalizes the result.
		 *
		 * @param {Matrix4} m - The matrix.
		 * @return {Vector3} A reference to this vector.
		 */
		transformDirection( m ) {

			// input: THREE.Matrix4 affine matrix
			// vector interpreted as a direction

			const x = this.x, y = this.y, z = this.z;
			const e = m.elements;

			this.x = e[ 0 ] * x + e[ 4 ] * y + e[ 8 ] * z;
			this.y = e[ 1 ] * x + e[ 5 ] * y + e[ 9 ] * z;
			this.z = e[ 2 ] * x + e[ 6 ] * y + e[ 10 ] * z;

			return this.normalize();

		}

		/**
		 * Divides this instance by the given vector.
		 *
		 * @param {Vector3} v - The vector to divide.
		 * @return {Vector3} A reference to this vector.
		 */
		divide( v ) {

			this.x /= v.x;
			this.y /= v.y;
			this.z /= v.z;

			return this;

		}

		/**
		 * Divides this vector by the given scalar.
		 *
		 * @param {number} scalar - The scalar to divide.
		 * @return {Vector3} A reference to this vector.
		 */
		divideScalar( scalar ) {

			return this.multiplyScalar( 1 / scalar );

		}

		/**
		 * If this vector's x, y or z value is greater than the given vector's x, y or z
		 * value, replace that value with the corresponding min value.
		 *
		 * @param {Vector3} v - The vector.
		 * @return {Vector3} A reference to this vector.
		 */
		min( v ) {

			this.x = Math.min( this.x, v.x );
			this.y = Math.min( this.y, v.y );
			this.z = Math.min( this.z, v.z );

			return this;

		}

		/**
		 * If this vector's x, y or z value is less than the given vector's x, y or z
		 * value, replace that value with the corresponding max value.
		 *
		 * @param {Vector3} v - The vector.
		 * @return {Vector3} A reference to this vector.
		 */
		max( v ) {

			this.x = Math.max( this.x, v.x );
			this.y = Math.max( this.y, v.y );
			this.z = Math.max( this.z, v.z );

			return this;

		}

		/**
		 * If this vector's x, y or z value is greater than the max vector's x, y or z
		 * value, it is replaced by the corresponding value.
		 * If this vector's x, y or z value is less than the min vector's x, y or z value,
		 * it is replaced by the corresponding value.
		 *
		 * @param {Vector3} min - The minimum x, y and z values.
		 * @param {Vector3} max - The maximum x, y and z values in the desired range.
		 * @return {Vector3} A reference to this vector.
		 */
		clamp( min, max ) {

			// assumes min < max, componentwise

			this.x = clamp( this.x, min.x, max.x );
			this.y = clamp( this.y, min.y, max.y );
			this.z = clamp( this.z, min.z, max.z );

			return this;

		}

		/**
		 * If this vector's x, y or z values are greater than the max value, they are
		 * replaced by the max value.
		 * If this vector's x, y or z values are less than the min value, they are
		 * replaced by the min value.
		 *
		 * @param {number} minVal - The minimum value the components will be clamped to.
		 * @param {number} maxVal - The maximum value the components will be clamped to.
		 * @return {Vector3} A reference to this vector.
		 */
		clampScalar( minVal, maxVal ) {

			this.x = clamp( this.x, minVal, maxVal );
			this.y = clamp( this.y, minVal, maxVal );
			this.z = clamp( this.z, minVal, maxVal );

			return this;

		}

		/**
		 * If this vector's length is greater than the max value, it is replaced by
		 * the max value.
		 * If this vector's length is less than the min value, it is replaced by the
		 * min value.
		 *
		 * @param {number} min - The minimum value the vector length will be clamped to.
		 * @param {number} max - The maximum value the vector length will be clamped to.
		 * @return {Vector3} A reference to this vector.
		 */
		clampLength( min, max ) {

			const length = this.length();

			return this.divideScalar( length || 1 ).multiplyScalar( clamp( length, min, max ) );

		}

		/**
		 * The components of this vector are rounded down to the nearest integer value.
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		floor() {

			this.x = Math.floor( this.x );
			this.y = Math.floor( this.y );
			this.z = Math.floor( this.z );

			return this;

		}

		/**
		 * The components of this vector are rounded up to the nearest integer value.
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		ceil() {

			this.x = Math.ceil( this.x );
			this.y = Math.ceil( this.y );
			this.z = Math.ceil( this.z );

			return this;

		}

		/**
		 * The components of this vector are rounded to the nearest integer value
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		round() {

			this.x = Math.round( this.x );
			this.y = Math.round( this.y );
			this.z = Math.round( this.z );

			return this;

		}

		/**
		 * The components of this vector are rounded towards zero (up if negative,
		 * down if positive) to an integer value.
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		roundToZero() {

			this.x = Math.trunc( this.x );
			this.y = Math.trunc( this.y );
			this.z = Math.trunc( this.z );

			return this;

		}

		/**
		 * Inverts this vector - i.e. sets x = -x, y = -y and z = -z.
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		negate() {

			this.x = - this.x;
			this.y = - this.y;
			this.z = - this.z;

			return this;

		}

		/**
		 * Calculates the dot product of the given vector with this instance.
		 *
		 * @param {Vector3} v - The vector to compute the dot product with.
		 * @return {number} The result of the dot product.
		 */
		dot( v ) {

			return this.x * v.x + this.y * v.y + this.z * v.z;

		}

		// TODO lengthSquared?

		/**
		 * Computes the square of the Euclidean length (straight-line length) from
		 * (0, 0, 0) to (x, y, z). If you are comparing the lengths of vectors, you should
		 * compare the length squared instead as it is slightly more efficient to calculate.
		 *
		 * @return {number} The square length of this vector.
		 */
		lengthSq() {

			return this.x * this.x + this.y * this.y + this.z * this.z;

		}

		/**
		 * Computes the  Euclidean length (straight-line length) from (0, 0, 0) to (x, y, z).
		 *
		 * @return {number} The length of this vector.
		 */
		length() {

			return Math.sqrt( this.x * this.x + this.y * this.y + this.z * this.z );

		}

		/**
		 * Computes the Manhattan length of this vector.
		 *
		 * @return {number} The length of this vector.
		 */
		manhattanLength() {

			return Math.abs( this.x ) + Math.abs( this.y ) + Math.abs( this.z );

		}

		/**
		 * Converts this vector to a unit vector - that is, sets it equal to a vector
		 * with the same direction as this one, but with a vector length of `1`.
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		normalize() {

			return this.divideScalar( this.length() || 1 );

		}

		/**
		 * Sets this vector to a vector with the same direction as this one, but
		 * with the specified length.
		 *
		 * @param {number} length - The new length of this vector.
		 * @return {Vector3} A reference to this vector.
		 */
		setLength( length ) {

			return this.normalize().multiplyScalar( length );

		}

		/**
		 * Linearly interpolates between the given vector and this instance, where
		 * alpha is the percent distance along the line - alpha = 0 will be this
		 * vector, and alpha = 1 will be the given one.
		 *
		 * @param {Vector3} v - The vector to interpolate towards.
		 * @param {number} alpha - The interpolation factor, typically in the closed interval `[0, 1]`.
		 * @return {Vector3} A reference to this vector.
		 */
		lerp( v, alpha ) {

			this.x += ( v.x - this.x ) * alpha;
			this.y += ( v.y - this.y ) * alpha;
			this.z += ( v.z - this.z ) * alpha;

			return this;

		}

		/**
		 * Linearly interpolates between the given vectors, where alpha is the percent
		 * distance along the line - alpha = 0 will be first vector, and alpha = 1 will
		 * be the second one. The result is stored in this instance.
		 *
		 * @param {Vector3} v1 - The first vector.
		 * @param {Vector3} v2 - The second vector.
		 * @param {number} alpha - The interpolation factor, typically in the closed interval `[0, 1]`.
		 * @return {Vector3} A reference to this vector.
		 */
		lerpVectors( v1, v2, alpha ) {

			this.x = v1.x + ( v2.x - v1.x ) * alpha;
			this.y = v1.y + ( v2.y - v1.y ) * alpha;
			this.z = v1.z + ( v2.z - v1.z ) * alpha;

			return this;

		}

		/**
		 * Calculates the cross product of the given vector with this instance.
		 *
		 * @param {Vector3} v - The vector to compute the cross product with.
		 * @return {Vector3} The result of the cross product.
		 */
		cross( v ) {

			return this.crossVectors( this, v );

		}

		/**
		 * Calculates the cross product of the given vectors and stores the result
		 * in this instance.
		 *
		 * @param {Vector3} a - The first vector.
		 * @param {Vector3} b - The second vector.
		 * @return {Vector3} A reference to this vector.
		 */
		crossVectors( a, b ) {

			const ax = a.x, ay = a.y, az = a.z;
			const bx = b.x, by = b.y, bz = b.z;

			this.x = ay * bz - az * by;
			this.y = az * bx - ax * bz;
			this.z = ax * by - ay * bx;

			return this;

		}

		/**
		 * Projects this vector onto the given one.
		 *
		 * @param {Vector3} v - The vector to project to.
		 * @return {Vector3} A reference to this vector.
		 */
		projectOnVector( v ) {

			const denominator = v.lengthSq();

			if ( denominator === 0 ) return this.set( 0, 0, 0 );

			const scalar = v.dot( this ) / denominator;

			return this.copy( v ).multiplyScalar( scalar );

		}

		/**
		 * Projects this vector onto a plane by subtracting this
		 * vector projected onto the plane's normal from this vector.
		 *
		 * @param {Vector3} planeNormal - The plane normal.
		 * @return {Vector3} A reference to this vector.
		 */
		projectOnPlane( planeNormal ) {

			_vector$4.copy( this ).projectOnVector( planeNormal );

			return this.sub( _vector$4 );

		}

		/**
		 * Reflects this vector off a plane orthogonal to the given normal vector.
		 *
		 * @param {Vector3} normal - The (normalized) normal vector.
		 * @return {Vector3} A reference to this vector.
		 */
		reflect( normal ) {

			return this.sub( _vector$4.copy( normal ).multiplyScalar( 2 * this.dot( normal ) ) );

		}
		/**
		 * Returns the angle between the given vector and this instance in radians.
		 *
		 * @param {Vector3} v - The vector to compute the angle with.
		 * @return {number} The angle in radians.
		 */
		angleTo( v ) {

			const denominator = Math.sqrt( this.lengthSq() * v.lengthSq() );

			if ( denominator === 0 ) return Math.PI / 2;

			const theta = this.dot( v ) / denominator;

			// clamp, to handle numerical problems

			return Math.acos( clamp( theta, - 1, 1 ) );

		}

		/**
		 * Computes the distance from the given vector to this instance.
		 *
		 * @param {Vector3} v - The vector to compute the distance to.
		 * @return {number} The distance.
		 */
		distanceTo( v ) {

			return Math.sqrt( this.distanceToSquared( v ) );

		}

		/**
		 * Computes the squared distance from the given vector to this instance.
		 * If you are just comparing the distance with another distance, you should compare
		 * the distance squared instead as it is slightly more efficient to calculate.
		 *
		 * @param {Vector3} v - The vector to compute the squared distance to.
		 * @return {number} The squared distance.
		 */
		distanceToSquared( v ) {

			const dx = this.x - v.x, dy = this.y - v.y, dz = this.z - v.z;

			return dx * dx + dy * dy + dz * dz;

		}

		/**
		 * Computes the Manhattan distance from the given vector to this instance.
		 *
		 * @param {Vector3} v - The vector to compute the Manhattan distance to.
		 * @return {number} The Manhattan distance.
		 */
		manhattanDistanceTo( v ) {

			return Math.abs( this.x - v.x ) + Math.abs( this.y - v.y ) + Math.abs( this.z - v.z );

		}

		/**
		 * Sets the vector components from the given spherical coordinates.
		 *
		 * @param {Spherical} s - The spherical coordinates.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromSpherical( s ) {

			return this.setFromSphericalCoords( s.radius, s.phi, s.theta );

		}

		/**
		 * Sets the vector components from the given spherical coordinates.
		 *
		 * @param {number} radius - The radius.
		 * @param {number} phi - The phi angle in radians.
		 * @param {number} theta - The theta angle in radians.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromSphericalCoords( radius, phi, theta ) {

			const sinPhiRadius = Math.sin( phi ) * radius;

			this.x = sinPhiRadius * Math.sin( theta );
			this.y = Math.cos( phi ) * radius;
			this.z = sinPhiRadius * Math.cos( theta );

			return this;

		}

		/**
		 * Sets the vector components from the given cylindrical coordinates.
		 *
		 * @param {Cylindrical} c - The cylindrical coordinates.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromCylindrical( c ) {

			return this.setFromCylindricalCoords( c.radius, c.theta, c.y );

		}

		/**
		 * Sets the vector components from the given cylindrical coordinates.
		 *
		 * @param {number} radius - The radius.
		 * @param {number} theta - The theta angle in radians.
		 * @param {number} y - The y value.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromCylindricalCoords( radius, theta, y ) {

			this.x = radius * Math.sin( theta );
			this.y = y;
			this.z = radius * Math.cos( theta );

			return this;

		}

		/**
		 * Sets the vector components to the position elements of the
		 * given transformation matrix.
		 *
		 * @param {Matrix4} m - The 4x4 matrix.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromMatrixPosition( m ) {

			const e = m.elements;

			this.x = e[ 12 ];
			this.y = e[ 13 ];
			this.z = e[ 14 ];

			return this;

		}

		/**
		 * Sets the vector components to the scale elements of the
		 * given transformation matrix.
		 *
		 * @param {Matrix4} m - The 4x4 matrix.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromMatrixScale( m ) {

			const sx = this.setFromMatrixColumn( m, 0 ).length();
			const sy = this.setFromMatrixColumn( m, 1 ).length();
			const sz = this.setFromMatrixColumn( m, 2 ).length();

			this.x = sx;
			this.y = sy;
			this.z = sz;

			return this;

		}

		/**
		 * Sets the vector components from the specified matrix column.
		 *
		 * @param {Matrix4} m - The 4x4 matrix.
		 * @param {number} index - The column index.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromMatrixColumn( m, index ) {

			return this.fromArray( m.elements, index * 4 );

		}

		/**
		 * Sets the vector components from the specified matrix column.
		 *
		 * @param {Matrix3} m - The 3x3 matrix.
		 * @param {number} index - The column index.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromMatrix3Column( m, index ) {

			return this.fromArray( m.elements, index * 3 );

		}

		/**
		 * Sets the vector components from the given Euler angles.
		 *
		 * @param {Euler} e - The Euler angles to set.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromEuler( e ) {

			this.x = e._x;
			this.y = e._y;
			this.z = e._z;

			return this;

		}

		/**
		 * Sets the vector components from the RGB components of the
		 * given color.
		 *
		 * @param {Color} c - The color to set.
		 * @return {Vector3} A reference to this vector.
		 */
		setFromColor( c ) {

			this.x = c.r;
			this.y = c.g;
			this.z = c.b;

			return this;

		}

		/**
		 * Returns `true` if this vector is equal with the given one.
		 *
		 * @param {Vector3} v - The vector to test for equality.
		 * @return {boolean} Whether this vector is equal with the given one.
		 */
		equals( v ) {

			return ( ( v.x === this.x ) && ( v.y === this.y ) && ( v.z === this.z ) );

		}

		/**
		 * Sets this vector's x value to be `array[ offset ]`, y value to be `array[ offset + 1 ]`
		 * and z value to be `array[ offset + 2 ]`.
		 *
		 * @param {Array<number>} array - An array holding the vector component values.
		 * @param {number} [offset=0] - The offset into the array.
		 * @return {Vector3} A reference to this vector.
		 */
		fromArray( array, offset = 0 ) {

			this.x = array[ offset ];
			this.y = array[ offset + 1 ];
			this.z = array[ offset + 2 ];

			return this;

		}

		/**
		 * Writes the components of this vector to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number>} [array=[]] - The target array holding the vector components.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number>} The vector components.
		 */
		toArray( array = [], offset = 0 ) {

			array[ offset ] = this.x;
			array[ offset + 1 ] = this.y;
			array[ offset + 2 ] = this.z;

			return array;

		}

		/**
		 * Sets the components of this vector from the given buffer attribute.
		 *
		 * @param {BufferAttribute} attribute - The buffer attribute holding vector data.
		 * @param {number} index - The index into the attribute.
		 * @return {Vector3} A reference to this vector.
		 */
		fromBufferAttribute( attribute, index ) {

			this.x = attribute.getX( index );
			this.y = attribute.getY( index );
			this.z = attribute.getZ( index );

			return this;

		}

		/**
		 * Sets each component of this vector to a pseudo-random value between `0` and
		 * `1`, excluding `1`.
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		random() {

			this.x = Math.random();
			this.y = Math.random();
			this.z = Math.random();

			return this;

		}

		/**
		 * Sets this vector to a uniformly random point on a unit sphere.
		 *
		 * @return {Vector3} A reference to this vector.
		 */
		randomDirection() {

			// https://mathworld.wolfram.com/SpherePointPicking.html

			const theta = Math.random() * Math.PI * 2;
			const u = Math.random() * 2 - 1;
			const c = Math.sqrt( 1 - u * u );

			this.x = c * Math.cos( theta );
			this.y = u;
			this.z = c * Math.sin( theta );

			return this;

		}

		*[ Symbol.iterator ]() {

			yield this.x;
			yield this.y;
			yield this.z;

		}

	}

	const _vector$4 = /*@__PURE__*/ new Vector3();
	const _quaternion$2 = /*@__PURE__*/ new Quaternion();

	/**
	 * Only front faces are rendered.
	 *
	 * @type {number}
	 * @constant
	 */
	const FrontSide = 0;

	/**
	 * Only back faces are rendered.
	 *
	 * @type {number}
	 * @constant
	 */
	const BackSide = 1;

	/**
	 * The default blending.
	 *
	 * @type {number}
	 * @constant
	 */
	const NormalBlending = 1;

	/**
	 * A `source + destination` blending equation.
	 *
	 * @type {number}
	 * @constant
	 */
	const AddEquation = 100;

	/**
	 * Multiplies all colors by the source alpha value.
	 *
	 * @type {number}
	 * @constant
	 */
	const SrcAlphaFactor = 204;

	/**
	 * Multiplies all colors by 1 minus the source alpha value.
	 *
	 * @type {number}
	 * @constant
	 */
	const OneMinusSrcAlphaFactor = 205;

	/**
	 * Pass if the incoming value is less than or equal to the depth buffer value.
	 *
	 * @type {number}
	 * @constant
	 */
	const LessEqualDepth = 3;

	/**
	 * Multiplies the environment map color with the surface color.
	 *
	 * @type {number}
	 * @constant
	 */
	const MultiplyOperation = 0;

	/**
	 * A float data type for textures.
	 *
	 * @type {number}
	 * @constant
	 */
	const FloatType = 1015;

	// Color space string identifiers, matching CSS Color Module Level 4 and WebGPU names where available.

	/**
	 * No color space.
	 *
	 * @type {string}
	 * @constant
	 */
	const NoColorSpace = '';

	/**
	 * sRGB color space.
	 *
	 * @type {string}
	 * @constant
	 */
	const SRGBColorSpace = 'srgb';

	/**
	 * sRGB-linear color space.
	 *
	 * @type {string}
	 * @constant
	 */
	const LinearSRGBColorSpace = 'srgb-linear';

	/**
	 * Linear transfer function.
	 *
	 * @type {string}
	 * @constant
	 */
	const LinearTransfer = 'linear';

	/**
	 * sRGB transfer function.
	 *
	 * @type {string}
	 * @constant
	 */
	const SRGBTransfer = 'srgb';

	/**
	 * Keeps the current value.
	 *
	 * @type {number}
	 * @constant
	 */
	const KeepStencilOp = 7680;

	/**
	 * Will always return true.
	 *
	 * @type {number}
	 * @constant
	 */
	const AlwaysStencilFunc = 519;

	/**
	 * The contents are intended to be specified once by the application, and used many
	 * times as the source for drawing and image specification commands.
	 *
	 * @type {number}
	 * @constant
	 */
	const StaticDrawUsage = 35044;

	/**
	 * WebGL coordinate system.
	 *
	 * @type {number}
	 * @constant
	 */
	const WebGLCoordinateSystem = 2000;

	/**
	 * WebGPU coordinate system.
	 *
	 * @type {number}
	 * @constant
	 */
	const WebGPUCoordinateSystem = 2001;

	/**
	 * This type represents mouse buttons and interaction types in context of controls.
	 *
	 * @typedef {Object} ConstantsMouse
	 * @property {number} MIDDLE - The left mouse button.
	 * @property {number} LEFT - The middle mouse button.
	 * @property {number} RIGHT - The right mouse button.
	 * @property {number} ROTATE - A rotate interaction.
	 * @property {number} DOLLY - A dolly interaction.
	 * @property {number} PAN - A pan interaction.
	 **/

	/**
	 * This type represents touch interaction types in context of controls.
	 *
	 * @typedef {Object} ConstantsTouch
	 * @property {number} ROTATE - A rotate interaction.
	 * @property {number} PAN - A pan interaction.
	 * @property {number} DOLLY_PAN - The dolly-pan interaction.
	 * @property {number} DOLLY_ROTATE - A dolly-rotate interaction.
	 **/

	/**
	 * This type represents the different timestamp query types.
	 *
	 * @typedef {Object} ConstantsTimestampQuery
	 * @property {string} COMPUTE - A `compute` timestamp query.
	 * @property {string} RENDER - A `render` timestamp query.
	 **/

	/**
	 * Represents the different interpolation sampling types.
	 *
	 * @typedef {Object} ConstantsInterpolationSamplingType
	 * @property {string} PERSPECTIVE - Perspective-correct interpolation.
	 * @property {string} LINEAR - Linear interpolation.
	 * @property {string} FLAT - Flat interpolation.
	 */

	/**
	 * Represents the different interpolation sampling modes.
	 *
	 * @typedef {Object} ConstantsInterpolationSamplingMode
	 * @property {string} NORMAL - Normal sampling mode.
	 * @property {string} CENTROID - Centroid sampling mode.
	 * @property {string} SAMPLE - Sample-specific sampling mode.
	 * @property {string} FIRST - Flat interpolation using the first vertex.
	 * @property {string} EITHER - Flat interpolation using either vertex.
	 */

	/**
	 * Represents a 4x4 matrix.
	 *
	 * The most common use of a 4x4 matrix in 3D computer graphics is as a transformation matrix.
	 * For an introduction to transformation matrices as used in WebGL, check out [this tutorial]{@link https://www.opengl-tutorial.org/beginners-tutorials/tutorial-3-matrices}
	 *
	 * This allows a 3D vector representing a point in 3D space to undergo
	 * transformations such as translation, rotation, shear, scale, reflection,
	 * orthogonal or perspective projection and so on, by being multiplied by the
	 * matrix. This is known as `applying` the matrix to the vector.
	 *
	 * A Note on Row-Major and Column-Major Ordering:
	 *
	 * The constructor and {@link Matrix3#set} method take arguments in
	 * [row-major]{@link https://en.wikipedia.org/wiki/Row-_and_column-major_order#Column-major_order}
	 * order, while internally they are stored in the {@link Matrix3#elements} array in column-major order.
	 * This means that calling:
	 * ```js
	 * const m = new THREE.Matrix4();
	 * m.set( 11, 12, 13, 14,
	 *        21, 22, 23, 24,
	 *        31, 32, 33, 34,
	 *        41, 42, 43, 44 );
	 * ```
	 * will result in the elements array containing:
	 * ```js
	 * m.elements = [ 11, 21, 31, 41,
	 *                12, 22, 32, 42,
	 *                13, 23, 33, 43,
	 *                14, 24, 34, 44 ];
	 * ```
	 * and internally all calculations are performed using column-major ordering.
	 * However, as the actual ordering makes no difference mathematically and
	 * most people are used to thinking about matrices in row-major order, the
	 * three.js documentation shows matrices in row-major order. Just bear in
	 * mind that if you are reading the source code, you'll have to take the
	 * transpose of any matrices outlined here to make sense of the calculations.
	 */
	class Matrix4 {

		/**
		 * Constructs a new 4x4 matrix. The arguments are supposed to be
		 * in row-major order. If no arguments are provided, the constructor
		 * initializes the matrix as an identity matrix.
		 *
		 * @param {number} [n11] - 1-1 matrix element.
		 * @param {number} [n12] - 1-2 matrix element.
		 * @param {number} [n13] - 1-3 matrix element.
		 * @param {number} [n14] - 1-4 matrix element.
		 * @param {number} [n21] - 2-1 matrix element.
		 * @param {number} [n22] - 2-2 matrix element.
		 * @param {number} [n23] - 2-3 matrix element.
		 * @param {number} [n24] - 2-4 matrix element.
		 * @param {number} [n31] - 3-1 matrix element.
		 * @param {number} [n32] - 3-2 matrix element.
		 * @param {number} [n33] - 3-3 matrix element.
		 * @param {number} [n34] - 3-4 matrix element.
		 * @param {number} [n41] - 4-1 matrix element.
		 * @param {number} [n42] - 4-2 matrix element.
		 * @param {number} [n43] - 4-3 matrix element.
		 * @param {number} [n44] - 4-4 matrix element.
		 */
		constructor( n11, n12, n13, n14, n21, n22, n23, n24, n31, n32, n33, n34, n41, n42, n43, n44 ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			Matrix4.prototype.isMatrix4 = true;

			/**
			 * A column-major list of matrix values.
			 *
			 * @type {Array<number>}
			 */
			this.elements = [

				1, 0, 0, 0,
				0, 1, 0, 0,
				0, 0, 1, 0,
				0, 0, 0, 1

			];

			if ( n11 !== undefined ) {

				this.set( n11, n12, n13, n14, n21, n22, n23, n24, n31, n32, n33, n34, n41, n42, n43, n44 );

			}

		}

		/**
		 * Sets the elements of the matrix.The arguments are supposed to be
		 * in row-major order.
		 *
		 * @param {number} [n11] - 1-1 matrix element.
		 * @param {number} [n12] - 1-2 matrix element.
		 * @param {number} [n13] - 1-3 matrix element.
		 * @param {number} [n14] - 1-4 matrix element.
		 * @param {number} [n21] - 2-1 matrix element.
		 * @param {number} [n22] - 2-2 matrix element.
		 * @param {number} [n23] - 2-3 matrix element.
		 * @param {number} [n24] - 2-4 matrix element.
		 * @param {number} [n31] - 3-1 matrix element.
		 * @param {number} [n32] - 3-2 matrix element.
		 * @param {number} [n33] - 3-3 matrix element.
		 * @param {number} [n34] - 3-4 matrix element.
		 * @param {number} [n41] - 4-1 matrix element.
		 * @param {number} [n42] - 4-2 matrix element.
		 * @param {number} [n43] - 4-3 matrix element.
		 * @param {number} [n44] - 4-4 matrix element.
		 * @return {Matrix4} A reference to this matrix.
		 */
		set( n11, n12, n13, n14, n21, n22, n23, n24, n31, n32, n33, n34, n41, n42, n43, n44 ) {

			const te = this.elements;

			te[ 0 ] = n11; te[ 4 ] = n12; te[ 8 ] = n13; te[ 12 ] = n14;
			te[ 1 ] = n21; te[ 5 ] = n22; te[ 9 ] = n23; te[ 13 ] = n24;
			te[ 2 ] = n31; te[ 6 ] = n32; te[ 10 ] = n33; te[ 14 ] = n34;
			te[ 3 ] = n41; te[ 7 ] = n42; te[ 11 ] = n43; te[ 15 ] = n44;

			return this;

		}

		/**
		 * Sets this matrix to the 4x4 identity matrix.
		 *
		 * @return {Matrix4} A reference to this matrix.
		 */
		identity() {

			this.set(

				1, 0, 0, 0,
				0, 1, 0, 0,
				0, 0, 1, 0,
				0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Returns a matrix with copied values from this instance.
		 *
		 * @return {Matrix4} A clone of this instance.
		 */
		clone() {

			return new Matrix4().fromArray( this.elements );

		}

		/**
		 * Copies the values of the given matrix to this instance.
		 *
		 * @param {Matrix4} m - The matrix to copy.
		 * @return {Matrix4} A reference to this matrix.
		 */
		copy( m ) {

			const te = this.elements;
			const me = m.elements;

			te[ 0 ] = me[ 0 ]; te[ 1 ] = me[ 1 ]; te[ 2 ] = me[ 2 ]; te[ 3 ] = me[ 3 ];
			te[ 4 ] = me[ 4 ]; te[ 5 ] = me[ 5 ]; te[ 6 ] = me[ 6 ]; te[ 7 ] = me[ 7 ];
			te[ 8 ] = me[ 8 ]; te[ 9 ] = me[ 9 ]; te[ 10 ] = me[ 10 ]; te[ 11 ] = me[ 11 ];
			te[ 12 ] = me[ 12 ]; te[ 13 ] = me[ 13 ]; te[ 14 ] = me[ 14 ]; te[ 15 ] = me[ 15 ];

			return this;

		}

		/**
		 * Copies the translation component of the given matrix
		 * into this matrix's translation component.
		 *
		 * @param {Matrix4} m - The matrix to copy the translation component.
		 * @return {Matrix4} A reference to this matrix.
		 */
		copyPosition( m ) {

			const te = this.elements, me = m.elements;

			te[ 12 ] = me[ 12 ];
			te[ 13 ] = me[ 13 ];
			te[ 14 ] = me[ 14 ];

			return this;

		}

		/**
		 * Set the upper 3x3 elements of this matrix to the values of given 3x3 matrix.
		 *
		 * @param {Matrix3} m - The 3x3 matrix.
		 * @return {Matrix4} A reference to this matrix.
		 */
		setFromMatrix3( m ) {

			const me = m.elements;

			this.set(

				me[ 0 ], me[ 3 ], me[ 6 ], 0,
				me[ 1 ], me[ 4 ], me[ 7 ], 0,
				me[ 2 ], me[ 5 ], me[ 8 ], 0,
				0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Extracts the basis of this matrix into the three axis vectors provided.
		 *
		 * @param {Vector3} xAxis - The basis's x axis.
		 * @param {Vector3} yAxis - The basis's y axis.
		 * @param {Vector3} zAxis - The basis's z axis.
		 * @return {Matrix4} A reference to this matrix.
		 */
		extractBasis( xAxis, yAxis, zAxis ) {

			xAxis.setFromMatrixColumn( this, 0 );
			yAxis.setFromMatrixColumn( this, 1 );
			zAxis.setFromMatrixColumn( this, 2 );

			return this;

		}

		/**
		 * Sets the given basis vectors to this matrix.
		 *
		 * @param {Vector3} xAxis - The basis's x axis.
		 * @param {Vector3} yAxis - The basis's y axis.
		 * @param {Vector3} zAxis - The basis's z axis.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeBasis( xAxis, yAxis, zAxis ) {

			this.set(
				xAxis.x, yAxis.x, zAxis.x, 0,
				xAxis.y, yAxis.y, zAxis.y, 0,
				xAxis.z, yAxis.z, zAxis.z, 0,
				0, 0, 0, 1
			);

			return this;

		}

		/**
		 * Extracts the rotation component of the given matrix
		 * into this matrix's rotation component.
		 *
		 * Note: This method does not support reflection matrices.
		 *
		 * @param {Matrix4} m - The matrix.
		 * @return {Matrix4} A reference to this matrix.
		 */
		extractRotation( m ) {

			const te = this.elements;
			const me = m.elements;

			const scaleX = 1 / _v1$4.setFromMatrixColumn( m, 0 ).length();
			const scaleY = 1 / _v1$4.setFromMatrixColumn( m, 1 ).length();
			const scaleZ = 1 / _v1$4.setFromMatrixColumn( m, 2 ).length();

			te[ 0 ] = me[ 0 ] * scaleX;
			te[ 1 ] = me[ 1 ] * scaleX;
			te[ 2 ] = me[ 2 ] * scaleX;
			te[ 3 ] = 0;

			te[ 4 ] = me[ 4 ] * scaleY;
			te[ 5 ] = me[ 5 ] * scaleY;
			te[ 6 ] = me[ 6 ] * scaleY;
			te[ 7 ] = 0;

			te[ 8 ] = me[ 8 ] * scaleZ;
			te[ 9 ] = me[ 9 ] * scaleZ;
			te[ 10 ] = me[ 10 ] * scaleZ;
			te[ 11 ] = 0;

			te[ 12 ] = 0;
			te[ 13 ] = 0;
			te[ 14 ] = 0;
			te[ 15 ] = 1;

			return this;

		}

		/**
		 * Sets the rotation component (the upper left 3x3 matrix) of this matrix to
		 * the rotation specified by the given Euler angles. The rest of
		 * the matrix is set to the identity. Depending on the {@link Euler#order},
		 * there are six possible outcomes. See [this page]{@link https://en.wikipedia.org/wiki/Euler_angles#Rotation_matrix}
		 * for a complete list.
		 *
		 * @param {Euler} euler - The Euler angles.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeRotationFromEuler( euler ) {

			const te = this.elements;

			const x = euler.x, y = euler.y, z = euler.z;
			const a = Math.cos( x ), b = Math.sin( x );
			const c = Math.cos( y ), d = Math.sin( y );
			const e = Math.cos( z ), f = Math.sin( z );

			if ( euler.order === 'XYZ' ) {

				const ae = a * e, af = a * f, be = b * e, bf = b * f;

				te[ 0 ] = c * e;
				te[ 4 ] = - c * f;
				te[ 8 ] = d;

				te[ 1 ] = af + be * d;
				te[ 5 ] = ae - bf * d;
				te[ 9 ] = - b * c;

				te[ 2 ] = bf - ae * d;
				te[ 6 ] = be + af * d;
				te[ 10 ] = a * c;

			} else if ( euler.order === 'YXZ' ) {

				const ce = c * e, cf = c * f, de = d * e, df = d * f;

				te[ 0 ] = ce + df * b;
				te[ 4 ] = de * b - cf;
				te[ 8 ] = a * d;

				te[ 1 ] = a * f;
				te[ 5 ] = a * e;
				te[ 9 ] = - b;

				te[ 2 ] = cf * b - de;
				te[ 6 ] = df + ce * b;
				te[ 10 ] = a * c;

			} else if ( euler.order === 'ZXY' ) {

				const ce = c * e, cf = c * f, de = d * e, df = d * f;

				te[ 0 ] = ce - df * b;
				te[ 4 ] = - a * f;
				te[ 8 ] = de + cf * b;

				te[ 1 ] = cf + de * b;
				te[ 5 ] = a * e;
				te[ 9 ] = df - ce * b;

				te[ 2 ] = - a * d;
				te[ 6 ] = b;
				te[ 10 ] = a * c;

			} else if ( euler.order === 'ZYX' ) {

				const ae = a * e, af = a * f, be = b * e, bf = b * f;

				te[ 0 ] = c * e;
				te[ 4 ] = be * d - af;
				te[ 8 ] = ae * d + bf;

				te[ 1 ] = c * f;
				te[ 5 ] = bf * d + ae;
				te[ 9 ] = af * d - be;

				te[ 2 ] = - d;
				te[ 6 ] = b * c;
				te[ 10 ] = a * c;

			} else if ( euler.order === 'YZX' ) {

				const ac = a * c, ad = a * d, bc = b * c, bd = b * d;

				te[ 0 ] = c * e;
				te[ 4 ] = bd - ac * f;
				te[ 8 ] = bc * f + ad;

				te[ 1 ] = f;
				te[ 5 ] = a * e;
				te[ 9 ] = - b * e;

				te[ 2 ] = - d * e;
				te[ 6 ] = ad * f + bc;
				te[ 10 ] = ac - bd * f;

			} else if ( euler.order === 'XZY' ) {

				const ac = a * c, ad = a * d, bc = b * c, bd = b * d;

				te[ 0 ] = c * e;
				te[ 4 ] = - f;
				te[ 8 ] = d * e;

				te[ 1 ] = ac * f + bd;
				te[ 5 ] = a * e;
				te[ 9 ] = ad * f - bc;

				te[ 2 ] = bc * f - ad;
				te[ 6 ] = b * e;
				te[ 10 ] = bd * f + ac;

			}

			// bottom row
			te[ 3 ] = 0;
			te[ 7 ] = 0;
			te[ 11 ] = 0;

			// last column
			te[ 12 ] = 0;
			te[ 13 ] = 0;
			te[ 14 ] = 0;
			te[ 15 ] = 1;

			return this;

		}

		/**
		 * Sets the rotation component of this matrix to the rotation specified by
		 * the given Quaternion as outlined [here]{@link https://en.wikipedia.org/wiki/Rotation_matrix#Quaternion}
		 * The rest of the matrix is set to the identity.
		 *
		 * @param {Quaternion} q - The Quaternion.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeRotationFromQuaternion( q ) {

			return this.compose( _zero, q, _one );

		}

		/**
		 * Sets the rotation component of the transformation matrix, looking from `eye` towards
		 * `target`, and oriented by the up-direction.
		 *
		 * @param {Vector3} eye - The eye vector.
		 * @param {Vector3} target - The target vector.
		 * @param {Vector3} up - The up vector.
		 * @return {Matrix4} A reference to this matrix.
		 */
		lookAt( eye, target, up ) {

			const te = this.elements;

			_z.subVectors( eye, target );

			if ( _z.lengthSq() === 0 ) {

				// eye and target are in the same position

				_z.z = 1;

			}

			_z.normalize();
			_x.crossVectors( up, _z );

			if ( _x.lengthSq() === 0 ) {

				// up and z are parallel

				if ( Math.abs( up.z ) === 1 ) {

					_z.x += 0.0001;

				} else {

					_z.z += 0.0001;

				}

				_z.normalize();
				_x.crossVectors( up, _z );

			}

			_x.normalize();
			_y.crossVectors( _z, _x );

			te[ 0 ] = _x.x; te[ 4 ] = _y.x; te[ 8 ] = _z.x;
			te[ 1 ] = _x.y; te[ 5 ] = _y.y; te[ 9 ] = _z.y;
			te[ 2 ] = _x.z; te[ 6 ] = _y.z; te[ 10 ] = _z.z;

			return this;

		}

		/**
		 * Post-multiplies this matrix by the given 4x4 matrix.
		 *
		 * @param {Matrix4} m - The matrix to multiply with.
		 * @return {Matrix4} A reference to this matrix.
		 */
		multiply( m ) {

			return this.multiplyMatrices( this, m );

		}

		/**
		 * Pre-multiplies this matrix by the given 4x4 matrix.
		 *
		 * @param {Matrix4} m - The matrix to multiply with.
		 * @return {Matrix4} A reference to this matrix.
		 */
		premultiply( m ) {

			return this.multiplyMatrices( m, this );

		}

		/**
		 * Multiples the given 4x4 matrices and stores the result
		 * in this matrix.
		 *
		 * @param {Matrix4} a - The first matrix.
		 * @param {Matrix4} b - The second matrix.
		 * @return {Matrix4} A reference to this matrix.
		 */
		multiplyMatrices( a, b ) {

			const ae = a.elements;
			const be = b.elements;
			const te = this.elements;

			const a11 = ae[ 0 ], a12 = ae[ 4 ], a13 = ae[ 8 ], a14 = ae[ 12 ];
			const a21 = ae[ 1 ], a22 = ae[ 5 ], a23 = ae[ 9 ], a24 = ae[ 13 ];
			const a31 = ae[ 2 ], a32 = ae[ 6 ], a33 = ae[ 10 ], a34 = ae[ 14 ];
			const a41 = ae[ 3 ], a42 = ae[ 7 ], a43 = ae[ 11 ], a44 = ae[ 15 ];

			const b11 = be[ 0 ], b12 = be[ 4 ], b13 = be[ 8 ], b14 = be[ 12 ];
			const b21 = be[ 1 ], b22 = be[ 5 ], b23 = be[ 9 ], b24 = be[ 13 ];
			const b31 = be[ 2 ], b32 = be[ 6 ], b33 = be[ 10 ], b34 = be[ 14 ];
			const b41 = be[ 3 ], b42 = be[ 7 ], b43 = be[ 11 ], b44 = be[ 15 ];

			te[ 0 ] = a11 * b11 + a12 * b21 + a13 * b31 + a14 * b41;
			te[ 4 ] = a11 * b12 + a12 * b22 + a13 * b32 + a14 * b42;
			te[ 8 ] = a11 * b13 + a12 * b23 + a13 * b33 + a14 * b43;
			te[ 12 ] = a11 * b14 + a12 * b24 + a13 * b34 + a14 * b44;

			te[ 1 ] = a21 * b11 + a22 * b21 + a23 * b31 + a24 * b41;
			te[ 5 ] = a21 * b12 + a22 * b22 + a23 * b32 + a24 * b42;
			te[ 9 ] = a21 * b13 + a22 * b23 + a23 * b33 + a24 * b43;
			te[ 13 ] = a21 * b14 + a22 * b24 + a23 * b34 + a24 * b44;

			te[ 2 ] = a31 * b11 + a32 * b21 + a33 * b31 + a34 * b41;
			te[ 6 ] = a31 * b12 + a32 * b22 + a33 * b32 + a34 * b42;
			te[ 10 ] = a31 * b13 + a32 * b23 + a33 * b33 + a34 * b43;
			te[ 14 ] = a31 * b14 + a32 * b24 + a33 * b34 + a34 * b44;

			te[ 3 ] = a41 * b11 + a42 * b21 + a43 * b31 + a44 * b41;
			te[ 7 ] = a41 * b12 + a42 * b22 + a43 * b32 + a44 * b42;
			te[ 11 ] = a41 * b13 + a42 * b23 + a43 * b33 + a44 * b43;
			te[ 15 ] = a41 * b14 + a42 * b24 + a43 * b34 + a44 * b44;

			return this;

		}

		/**
		 * Multiplies every component of the matrix by the given scalar.
		 *
		 * @param {number} s - The scalar.
		 * @return {Matrix4} A reference to this matrix.
		 */
		multiplyScalar( s ) {

			const te = this.elements;

			te[ 0 ] *= s; te[ 4 ] *= s; te[ 8 ] *= s; te[ 12 ] *= s;
			te[ 1 ] *= s; te[ 5 ] *= s; te[ 9 ] *= s; te[ 13 ] *= s;
			te[ 2 ] *= s; te[ 6 ] *= s; te[ 10 ] *= s; te[ 14 ] *= s;
			te[ 3 ] *= s; te[ 7 ] *= s; te[ 11 ] *= s; te[ 15 ] *= s;

			return this;

		}

		/**
		 * Computes and returns the determinant of this matrix.
		 *
		 * Based on the method outlined [here]{@link http://www.euclideanspace.com/maths/algebra/matrix/functions/inverse/fourD/index.html}.
		 *
		 * @return {number} The determinant.
		 */
		determinant() {

			const te = this.elements;

			const n11 = te[ 0 ], n12 = te[ 4 ], n13 = te[ 8 ], n14 = te[ 12 ];
			const n21 = te[ 1 ], n22 = te[ 5 ], n23 = te[ 9 ], n24 = te[ 13 ];
			const n31 = te[ 2 ], n32 = te[ 6 ], n33 = te[ 10 ], n34 = te[ 14 ];
			const n41 = te[ 3 ], n42 = te[ 7 ], n43 = te[ 11 ], n44 = te[ 15 ];

			//TODO: make this more efficient

			return (
				n41 * (
					+ n14 * n23 * n32
					 - n13 * n24 * n32
					 - n14 * n22 * n33
					 + n12 * n24 * n33
					 + n13 * n22 * n34
					 - n12 * n23 * n34
				) +
				n42 * (
					+ n11 * n23 * n34
					 - n11 * n24 * n33
					 + n14 * n21 * n33
					 - n13 * n21 * n34
					 + n13 * n24 * n31
					 - n14 * n23 * n31
				) +
				n43 * (
					+ n11 * n24 * n32
					 - n11 * n22 * n34
					 - n14 * n21 * n32
					 + n12 * n21 * n34
					 + n14 * n22 * n31
					 - n12 * n24 * n31
				) +
				n44 * (
					- n13 * n22 * n31
					 - n11 * n23 * n32
					 + n11 * n22 * n33
					 + n13 * n21 * n32
					 - n12 * n21 * n33
					 + n12 * n23 * n31
				)

			);

		}

		/**
		 * Transposes this matrix in place.
		 *
		 * @return {Matrix4} A reference to this matrix.
		 */
		transpose() {

			const te = this.elements;
			let tmp;

			tmp = te[ 1 ]; te[ 1 ] = te[ 4 ]; te[ 4 ] = tmp;
			tmp = te[ 2 ]; te[ 2 ] = te[ 8 ]; te[ 8 ] = tmp;
			tmp = te[ 6 ]; te[ 6 ] = te[ 9 ]; te[ 9 ] = tmp;

			tmp = te[ 3 ]; te[ 3 ] = te[ 12 ]; te[ 12 ] = tmp;
			tmp = te[ 7 ]; te[ 7 ] = te[ 13 ]; te[ 13 ] = tmp;
			tmp = te[ 11 ]; te[ 11 ] = te[ 14 ]; te[ 14 ] = tmp;

			return this;

		}

		/**
		 * Sets the position component for this matrix from the given vector,
		 * without affecting the rest of the matrix.
		 *
		 * @param {number|Vector3} x - The x component of the vector or alternatively the vector object.
		 * @param {number} y - The y component of the vector.
		 * @param {number} z - The z component of the vector.
		 * @return {Matrix4} A reference to this matrix.
		 */
		setPosition( x, y, z ) {

			const te = this.elements;

			if ( x.isVector3 ) {

				te[ 12 ] = x.x;
				te[ 13 ] = x.y;
				te[ 14 ] = x.z;

			} else {

				te[ 12 ] = x;
				te[ 13 ] = y;
				te[ 14 ] = z;

			}

			return this;

		}

		/**
		 * Inverts this matrix, using the [analytic method]{@link https://en.wikipedia.org/wiki/Invertible_matrix#Analytic_solution}.
		 * You can not invert with a determinant of zero. If you attempt this, the method produces
		 * a zero matrix instead.
		 *
		 * @return {Matrix4} A reference to this matrix.
		 */
		invert() {

			// based on http://www.euclideanspace.com/maths/algebra/matrix/functions/inverse/fourD/index.htm
			const te = this.elements,

				n11 = te[ 0 ], n21 = te[ 1 ], n31 = te[ 2 ], n41 = te[ 3 ],
				n12 = te[ 4 ], n22 = te[ 5 ], n32 = te[ 6 ], n42 = te[ 7 ],
				n13 = te[ 8 ], n23 = te[ 9 ], n33 = te[ 10 ], n43 = te[ 11 ],
				n14 = te[ 12 ], n24 = te[ 13 ], n34 = te[ 14 ], n44 = te[ 15 ],

				t11 = n23 * n34 * n42 - n24 * n33 * n42 + n24 * n32 * n43 - n22 * n34 * n43 - n23 * n32 * n44 + n22 * n33 * n44,
				t12 = n14 * n33 * n42 - n13 * n34 * n42 - n14 * n32 * n43 + n12 * n34 * n43 + n13 * n32 * n44 - n12 * n33 * n44,
				t13 = n13 * n24 * n42 - n14 * n23 * n42 + n14 * n22 * n43 - n12 * n24 * n43 - n13 * n22 * n44 + n12 * n23 * n44,
				t14 = n14 * n23 * n32 - n13 * n24 * n32 - n14 * n22 * n33 + n12 * n24 * n33 + n13 * n22 * n34 - n12 * n23 * n34;

			const det = n11 * t11 + n21 * t12 + n31 * t13 + n41 * t14;

			if ( det === 0 ) return this.set( 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 );

			const detInv = 1 / det;

			te[ 0 ] = t11 * detInv;
			te[ 1 ] = ( n24 * n33 * n41 - n23 * n34 * n41 - n24 * n31 * n43 + n21 * n34 * n43 + n23 * n31 * n44 - n21 * n33 * n44 ) * detInv;
			te[ 2 ] = ( n22 * n34 * n41 - n24 * n32 * n41 + n24 * n31 * n42 - n21 * n34 * n42 - n22 * n31 * n44 + n21 * n32 * n44 ) * detInv;
			te[ 3 ] = ( n23 * n32 * n41 - n22 * n33 * n41 - n23 * n31 * n42 + n21 * n33 * n42 + n22 * n31 * n43 - n21 * n32 * n43 ) * detInv;

			te[ 4 ] = t12 * detInv;
			te[ 5 ] = ( n13 * n34 * n41 - n14 * n33 * n41 + n14 * n31 * n43 - n11 * n34 * n43 - n13 * n31 * n44 + n11 * n33 * n44 ) * detInv;
			te[ 6 ] = ( n14 * n32 * n41 - n12 * n34 * n41 - n14 * n31 * n42 + n11 * n34 * n42 + n12 * n31 * n44 - n11 * n32 * n44 ) * detInv;
			te[ 7 ] = ( n12 * n33 * n41 - n13 * n32 * n41 + n13 * n31 * n42 - n11 * n33 * n42 - n12 * n31 * n43 + n11 * n32 * n43 ) * detInv;

			te[ 8 ] = t13 * detInv;
			te[ 9 ] = ( n14 * n23 * n41 - n13 * n24 * n41 - n14 * n21 * n43 + n11 * n24 * n43 + n13 * n21 * n44 - n11 * n23 * n44 ) * detInv;
			te[ 10 ] = ( n12 * n24 * n41 - n14 * n22 * n41 + n14 * n21 * n42 - n11 * n24 * n42 - n12 * n21 * n44 + n11 * n22 * n44 ) * detInv;
			te[ 11 ] = ( n13 * n22 * n41 - n12 * n23 * n41 - n13 * n21 * n42 + n11 * n23 * n42 + n12 * n21 * n43 - n11 * n22 * n43 ) * detInv;

			te[ 12 ] = t14 * detInv;
			te[ 13 ] = ( n13 * n24 * n31 - n14 * n23 * n31 + n14 * n21 * n33 - n11 * n24 * n33 - n13 * n21 * n34 + n11 * n23 * n34 ) * detInv;
			te[ 14 ] = ( n14 * n22 * n31 - n12 * n24 * n31 - n14 * n21 * n32 + n11 * n24 * n32 + n12 * n21 * n34 - n11 * n22 * n34 ) * detInv;
			te[ 15 ] = ( n12 * n23 * n31 - n13 * n22 * n31 + n13 * n21 * n32 - n11 * n23 * n32 - n12 * n21 * n33 + n11 * n22 * n33 ) * detInv;

			return this;

		}

		/**
		 * Multiplies the columns of this matrix by the given vector.
		 *
		 * @param {Vector3} v - The scale vector.
		 * @return {Matrix4} A reference to this matrix.
		 */
		scale( v ) {

			const te = this.elements;
			const x = v.x, y = v.y, z = v.z;

			te[ 0 ] *= x; te[ 4 ] *= y; te[ 8 ] *= z;
			te[ 1 ] *= x; te[ 5 ] *= y; te[ 9 ] *= z;
			te[ 2 ] *= x; te[ 6 ] *= y; te[ 10 ] *= z;
			te[ 3 ] *= x; te[ 7 ] *= y; te[ 11 ] *= z;

			return this;

		}

		/**
		 * Gets the maximum scale value of the three axes.
		 *
		 * @return {number} The maximum scale.
		 */
		getMaxScaleOnAxis() {

			const te = this.elements;

			const scaleXSq = te[ 0 ] * te[ 0 ] + te[ 1 ] * te[ 1 ] + te[ 2 ] * te[ 2 ];
			const scaleYSq = te[ 4 ] * te[ 4 ] + te[ 5 ] * te[ 5 ] + te[ 6 ] * te[ 6 ];
			const scaleZSq = te[ 8 ] * te[ 8 ] + te[ 9 ] * te[ 9 ] + te[ 10 ] * te[ 10 ];

			return Math.sqrt( Math.max( scaleXSq, scaleYSq, scaleZSq ) );

		}

		/**
		 * Sets this matrix as a translation transform from the given vector.
		 *
		 * @param {number|Vector3} x - The amount to translate in the X axis or alternatively a translation vector.
		 * @param {number} y - The amount to translate in the Y axis.
		 * @param {number} z - The amount to translate in the z axis.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeTranslation( x, y, z ) {

			if ( x.isVector3 ) {

				this.set(

					1, 0, 0, x.x,
					0, 1, 0, x.y,
					0, 0, 1, x.z,
					0, 0, 0, 1

				);

			} else {

				this.set(

					1, 0, 0, x,
					0, 1, 0, y,
					0, 0, 1, z,
					0, 0, 0, 1

				);

			}

			return this;

		}

		/**
		 * Sets this matrix as a rotational transformation around the X axis by
		 * the given angle.
		 *
		 * @param {number} theta - The rotation in radians.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeRotationX( theta ) {

			const c = Math.cos( theta ), s = Math.sin( theta );

			this.set(

				1, 0, 0, 0,
				0, c, - s, 0,
				0, s, c, 0,
				0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Sets this matrix as a rotational transformation around the Y axis by
		 * the given angle.
		 *
		 * @param {number} theta - The rotation in radians.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeRotationY( theta ) {

			const c = Math.cos( theta ), s = Math.sin( theta );

			this.set(

				 c, 0, s, 0,
				 0, 1, 0, 0,
				- s, 0, c, 0,
				 0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Sets this matrix as a rotational transformation around the Z axis by
		 * the given angle.
		 *
		 * @param {number} theta - The rotation in radians.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeRotationZ( theta ) {

			const c = Math.cos( theta ), s = Math.sin( theta );

			this.set(

				c, - s, 0, 0,
				s, c, 0, 0,
				0, 0, 1, 0,
				0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Sets this matrix as a rotational transformation around the given axis by
		 * the given angle.
		 *
		 * This is a somewhat controversial but mathematically sound alternative to
		 * rotating via Quaternions. See the discussion [here]{@link https://www.gamedev.net/articles/programming/math-and-physics/do-we-really-need-quaternions-r1199}.
		 *
		 * @param {Vector3} axis - The normalized rotation axis.
		 * @param {number} angle - The rotation in radians.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeRotationAxis( axis, angle ) {

			// Based on http://www.gamedev.net/reference/articles/article1199.asp

			const c = Math.cos( angle );
			const s = Math.sin( angle );
			const t = 1 - c;
			const x = axis.x, y = axis.y, z = axis.z;
			const tx = t * x, ty = t * y;

			this.set(

				tx * x + c, tx * y - s * z, tx * z + s * y, 0,
				tx * y + s * z, ty * y + c, ty * z - s * x, 0,
				tx * z - s * y, ty * z + s * x, t * z * z + c, 0,
				0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Sets this matrix as a scale transformation.
		 *
		 * @param {number} x - The amount to scale in the X axis.
		 * @param {number} y - The amount to scale in the Y axis.
		 * @param {number} z - The amount to scale in the Z axis.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeScale( x, y, z ) {

			this.set(

				x, 0, 0, 0,
				0, y, 0, 0,
				0, 0, z, 0,
				0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Sets this matrix as a shear transformation.
		 *
		 * @param {number} xy - The amount to shear X by Y.
		 * @param {number} xz - The amount to shear X by Z.
		 * @param {number} yx - The amount to shear Y by X.
		 * @param {number} yz - The amount to shear Y by Z.
		 * @param {number} zx - The amount to shear Z by X.
		 * @param {number} zy - The amount to shear Z by Y.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeShear( xy, xz, yx, yz, zx, zy ) {

			this.set(

				1, yx, zx, 0,
				xy, 1, zy, 0,
				xz, yz, 1, 0,
				0, 0, 0, 1

			);

			return this;

		}

		/**
		 * Sets this matrix to the transformation composed of the given position,
		 * rotation (Quaternion) and scale.
		 *
		 * @param {Vector3} position - The position vector.
		 * @param {Quaternion} quaternion - The rotation as a Quaternion.
		 * @param {Vector3} scale - The scale vector.
		 * @return {Matrix4} A reference to this matrix.
		 */
		compose( position, quaternion, scale ) {

			const te = this.elements;

			const x = quaternion._x, y = quaternion._y, z = quaternion._z, w = quaternion._w;
			const x2 = x + x,	y2 = y + y, z2 = z + z;
			const xx = x * x2, xy = x * y2, xz = x * z2;
			const yy = y * y2, yz = y * z2, zz = z * z2;
			const wx = w * x2, wy = w * y2, wz = w * z2;

			const sx = scale.x, sy = scale.y, sz = scale.z;

			te[ 0 ] = ( 1 - ( yy + zz ) ) * sx;
			te[ 1 ] = ( xy + wz ) * sx;
			te[ 2 ] = ( xz - wy ) * sx;
			te[ 3 ] = 0;

			te[ 4 ] = ( xy - wz ) * sy;
			te[ 5 ] = ( 1 - ( xx + zz ) ) * sy;
			te[ 6 ] = ( yz + wx ) * sy;
			te[ 7 ] = 0;

			te[ 8 ] = ( xz + wy ) * sz;
			te[ 9 ] = ( yz - wx ) * sz;
			te[ 10 ] = ( 1 - ( xx + yy ) ) * sz;
			te[ 11 ] = 0;

			te[ 12 ] = position.x;
			te[ 13 ] = position.y;
			te[ 14 ] = position.z;
			te[ 15 ] = 1;

			return this;

		}

		/**
		 * Decomposes this matrix into its position, rotation and scale components
		 * and provides the result in the given objects.
		 *
		 * Note: Not all matrices are decomposable in this way. For example, if an
		 * object has a non-uniformly scaled parent, then the object's world matrix
		 * may not be decomposable, and this method may not be appropriate.
		 *
		 * @param {Vector3} position - The position vector.
		 * @param {Quaternion} quaternion - The rotation as a Quaternion.
		 * @param {Vector3} scale - The scale vector.
		 * @return {Matrix4} A reference to this matrix.
		 */
		decompose( position, quaternion, scale ) {

			const te = this.elements;

			let sx = _v1$4.set( te[ 0 ], te[ 1 ], te[ 2 ] ).length();
			const sy = _v1$4.set( te[ 4 ], te[ 5 ], te[ 6 ] ).length();
			const sz = _v1$4.set( te[ 8 ], te[ 9 ], te[ 10 ] ).length();

			// if determine is negative, we need to invert one scale
			const det = this.determinant();
			if ( det < 0 ) sx = - sx;

			position.x = te[ 12 ];
			position.y = te[ 13 ];
			position.z = te[ 14 ];

			// scale the rotation part
			_m1$2.copy( this );

			const invSX = 1 / sx;
			const invSY = 1 / sy;
			const invSZ = 1 / sz;

			_m1$2.elements[ 0 ] *= invSX;
			_m1$2.elements[ 1 ] *= invSX;
			_m1$2.elements[ 2 ] *= invSX;

			_m1$2.elements[ 4 ] *= invSY;
			_m1$2.elements[ 5 ] *= invSY;
			_m1$2.elements[ 6 ] *= invSY;

			_m1$2.elements[ 8 ] *= invSZ;
			_m1$2.elements[ 9 ] *= invSZ;
			_m1$2.elements[ 10 ] *= invSZ;

			quaternion.setFromRotationMatrix( _m1$2 );

			scale.x = sx;
			scale.y = sy;
			scale.z = sz;

			return this;

		}

		/**
		 * Creates a perspective projection matrix. This is used internally by
		 * {@link PerspectiveCamera#updateProjectionMatrix}.

		 * @param {number} left - Left boundary of the viewing frustum at the near plane.
		 * @param {number} right - Right boundary of the viewing frustum at the near plane.
		 * @param {number} top - Top boundary of the viewing frustum at the near plane.
		 * @param {number} bottom - Bottom boundary of the viewing frustum at the near plane.
		 * @param {number} near - The distance from the camera to the near plane.
		 * @param {number} far - The distance from the camera to the far plane.
		 * @param {(WebGLCoordinateSystem|WebGPUCoordinateSystem)} [coordinateSystem=WebGLCoordinateSystem] - The coordinate system.
		 * @param {boolean} [reversedDepth=false] - Whether to use a reversed depth.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makePerspective( left, right, top, bottom, near, far, coordinateSystem = WebGLCoordinateSystem, reversedDepth = false ) {

			const te = this.elements;

			const x = 2 * near / ( right - left );
			const y = 2 * near / ( top - bottom );

			const a = ( right + left ) / ( right - left );
			const b = ( top + bottom ) / ( top - bottom );

			let c, d;

			if ( reversedDepth ) {

				c = near / ( far - near );
				d = ( far * near ) / ( far - near );

			} else {

				if ( coordinateSystem === WebGLCoordinateSystem ) {

					c = - ( far + near ) / ( far - near );
					d = ( - 2 * far * near ) / ( far - near );

				} else if ( coordinateSystem === WebGPUCoordinateSystem ) {

					c = - far / ( far - near );
					d = ( - far * near ) / ( far - near );

				} else {

					throw new Error( 'THREE.Matrix4.makePerspective(): Invalid coordinate system: ' + coordinateSystem );

				}

			}

			te[ 0 ] = x;	te[ 4 ] = 0;	te[ 8 ] = a; 	te[ 12 ] = 0;
			te[ 1 ] = 0;	te[ 5 ] = y;	te[ 9 ] = b; 	te[ 13 ] = 0;
			te[ 2 ] = 0;	te[ 6 ] = 0;	te[ 10 ] = c; 	te[ 14 ] = d;
			te[ 3 ] = 0;	te[ 7 ] = 0;	te[ 11 ] = - 1;	te[ 15 ] = 0;

			return this;

		}

		/**
		 * Creates a orthographic projection matrix. This is used internally by
		 * {@link OrthographicCamera#updateProjectionMatrix}.

		 * @param {number} left - Left boundary of the viewing frustum at the near plane.
		 * @param {number} right - Right boundary of the viewing frustum at the near plane.
		 * @param {number} top - Top boundary of the viewing frustum at the near plane.
		 * @param {number} bottom - Bottom boundary of the viewing frustum at the near plane.
		 * @param {number} near - The distance from the camera to the near plane.
		 * @param {number} far - The distance from the camera to the far plane.
		 * @param {(WebGLCoordinateSystem|WebGPUCoordinateSystem)} [coordinateSystem=WebGLCoordinateSystem] - The coordinate system.
		 * @param {boolean} [reversedDepth=false] - Whether to use a reversed depth.
		 * @return {Matrix4} A reference to this matrix.
		 */
		makeOrthographic( left, right, top, bottom, near, far, coordinateSystem = WebGLCoordinateSystem, reversedDepth = false ) {

			const te = this.elements;

			const x = 2 / ( right - left );
			const y = 2 / ( top - bottom );

			const a = - ( right + left ) / ( right - left );
			const b = - ( top + bottom ) / ( top - bottom );

			let c, d;

			if ( reversedDepth ) {

				c = 1 / ( far - near );
				d = far / ( far - near );

			} else {

				if ( coordinateSystem === WebGLCoordinateSystem ) {

					c = - 2 / ( far - near );
					d = - ( far + near ) / ( far - near );

				} else if ( coordinateSystem === WebGPUCoordinateSystem ) {

					c = - 1 / ( far - near );
					d = - near / ( far - near );

				} else {

					throw new Error( 'THREE.Matrix4.makeOrthographic(): Invalid coordinate system: ' + coordinateSystem );

				}

			}

			te[ 0 ] = x;		te[ 4 ] = 0;		te[ 8 ] = 0; 		te[ 12 ] = a;
			te[ 1 ] = 0; 		te[ 5 ] = y;		te[ 9 ] = 0; 		te[ 13 ] = b;
			te[ 2 ] = 0; 		te[ 6 ] = 0;		te[ 10 ] = c;		te[ 14 ] = d;
			te[ 3 ] = 0; 		te[ 7 ] = 0;		te[ 11 ] = 0;		te[ 15 ] = 1;

			return this;

		}

		/**
		 * Returns `true` if this matrix is equal with the given one.
		 *
		 * @param {Matrix4} matrix - The matrix to test for equality.
		 * @return {boolean} Whether this matrix is equal with the given one.
		 */
		equals( matrix ) {

			const te = this.elements;
			const me = matrix.elements;

			for ( let i = 0; i < 16; i ++ ) {

				if ( te[ i ] !== me[ i ] ) return false;

			}

			return true;

		}

		/**
		 * Sets the elements of the matrix from the given array.
		 *
		 * @param {Array<number>} array - The matrix elements in column-major order.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Matrix4} A reference to this matrix.
		 */
		fromArray( array, offset = 0 ) {

			for ( let i = 0; i < 16; i ++ ) {

				this.elements[ i ] = array[ i + offset ];

			}

			return this;

		}

		/**
		 * Writes the elements of this matrix to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number>} [array=[]] - The target array holding the matrix elements in column-major order.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number>} The matrix elements in column-major order.
		 */
		toArray( array = [], offset = 0 ) {

			const te = this.elements;

			array[ offset ] = te[ 0 ];
			array[ offset + 1 ] = te[ 1 ];
			array[ offset + 2 ] = te[ 2 ];
			array[ offset + 3 ] = te[ 3 ];

			array[ offset + 4 ] = te[ 4 ];
			array[ offset + 5 ] = te[ 5 ];
			array[ offset + 6 ] = te[ 6 ];
			array[ offset + 7 ] = te[ 7 ];

			array[ offset + 8 ] = te[ 8 ];
			array[ offset + 9 ] = te[ 9 ];
			array[ offset + 10 ] = te[ 10 ];
			array[ offset + 11 ] = te[ 11 ];

			array[ offset + 12 ] = te[ 12 ];
			array[ offset + 13 ] = te[ 13 ];
			array[ offset + 14 ] = te[ 14 ];
			array[ offset + 15 ] = te[ 15 ];

			return array;

		}

	}

	const _v1$4 = /*@__PURE__*/ new Vector3();
	const _m1$2 = /*@__PURE__*/ new Matrix4();
	const _zero = /*@__PURE__*/ new Vector3( 0, 0, 0 );
	const _one = /*@__PURE__*/ new Vector3( 1, 1, 1 );
	const _x = /*@__PURE__*/ new Vector3();
	const _y = /*@__PURE__*/ new Vector3();
	const _z = /*@__PURE__*/ new Vector3();

	/**
	 * Represents an axis-aligned bounding box (AABB) in 3D space.
	 */
	class Box3 {

		/**
		 * Constructs a new bounding box.
		 *
		 * @param {Vector3} [min=(Infinity,Infinity,Infinity)] - A vector representing the lower boundary of the box.
		 * @param {Vector3} [max=(-Infinity,-Infinity,-Infinity)] - A vector representing the upper boundary of the box.
		 */
		constructor( min = new Vector3( + Infinity, + Infinity, + Infinity ), max = new Vector3( - Infinity, - Infinity, - Infinity ) ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isBox3 = true;

			/**
			 * The lower boundary of the box.
			 *
			 * @type {Vector3}
			 */
			this.min = min;

			/**
			 * The upper boundary of the box.
			 *
			 * @type {Vector3}
			 */
			this.max = max;

		}

		/**
		 * Sets the lower and upper boundaries of this box.
		 * Please note that this method only copies the values from the given objects.
		 *
		 * @param {Vector3} min - The lower boundary of the box.
		 * @param {Vector3} max - The upper boundary of the box.
		 * @return {Box3} A reference to this bounding box.
		 */
		set( min, max ) {

			this.min.copy( min );
			this.max.copy( max );

			return this;

		}

		/**
		 * Sets the upper and lower bounds of this box so it encloses the position data
		 * in the given array.
		 *
		 * @param {Array<number>} array - An array holding 3D position data.
		 * @return {Box3} A reference to this bounding box.
		 */
		setFromArray( array ) {

			this.makeEmpty();

			for ( let i = 0, il = array.length; i < il; i += 3 ) {

				this.expandByPoint( _vector$3.fromArray( array, i ) );

			}

			return this;

		}

		/**
		 * Sets the upper and lower bounds of this box so it encloses the position data
		 * in the given buffer attribute.
		 *
		 * @param {BufferAttribute} attribute - A buffer attribute holding 3D position data.
		 * @return {Box3} A reference to this bounding box.
		 */
		setFromBufferAttribute( attribute ) {

			this.makeEmpty();

			for ( let i = 0, il = attribute.count; i < il; i ++ ) {

				this.expandByPoint( _vector$3.fromBufferAttribute( attribute, i ) );

			}

			return this;

		}

		/**
		 * Sets the upper and lower bounds of this box so it encloses the position data
		 * in the given array.
		 *
		 * @param {Array<Vector3>} points - An array holding 3D position data as instances of {@link Vector3}.
		 * @return {Box3} A reference to this bounding box.
		 */
		setFromPoints( points ) {

			this.makeEmpty();

			for ( let i = 0, il = points.length; i < il; i ++ ) {

				this.expandByPoint( points[ i ] );

			}

			return this;

		}

		/**
		 * Centers this box on the given center vector and sets this box's width, height and
		 * depth to the given size values.
		 *
		 * @param {Vector3} center - The center of the box.
		 * @param {Vector3} size - The x, y and z dimensions of the box.
		 * @return {Box3} A reference to this bounding box.
		 */
		setFromCenterAndSize( center, size ) {

			const halfSize = _vector$3.copy( size ).multiplyScalar( 0.5 );

			this.min.copy( center ).sub( halfSize );
			this.max.copy( center ).add( halfSize );

			return this;

		}

		/**
		 * Computes the world-axis-aligned bounding box for the given 3D object
		 * (including its children), accounting for the object's, and children's,
		 * world transforms. The function may result in a larger box than strictly necessary.
		 *
		 * @param {Object3D} object - The 3D object to compute the bounding box for.
		 * @param {boolean} [precise=false] - If set to `true`, the method computes the smallest
		 * world-axis-aligned bounding box at the expense of more computation.
		 * @return {Box3} A reference to this bounding box.
		 */
		setFromObject( object, precise = false ) {

			this.makeEmpty();

			return this.expandByObject( object, precise );

		}

		/**
		 * Returns a new box with copied values from this instance.
		 *
		 * @return {Box3} A clone of this instance.
		 */
		clone() {

			return new this.constructor().copy( this );

		}

		/**
		 * Copies the values of the given box to this instance.
		 *
		 * @param {Box3} box - The box to copy.
		 * @return {Box3} A reference to this bounding box.
		 */
		copy( box ) {

			this.min.copy( box.min );
			this.max.copy( box.max );

			return this;

		}

		/**
		 * Makes this box empty which means in encloses a zero space in 3D.
		 *
		 * @return {Box3} A reference to this bounding box.
		 */
		makeEmpty() {

			this.min.x = this.min.y = this.min.z = + Infinity;
			this.max.x = this.max.y = this.max.z = - Infinity;

			return this;

		}

		/**
		 * Returns true if this box includes zero points within its bounds.
		 * Note that a box with equal lower and upper bounds still includes one
		 * point, the one both bounds share.
		 *
		 * @return {boolean} Whether this box is empty or not.
		 */
		isEmpty() {

			// this is a more robust check for empty than ( volume <= 0 ) because volume can get positive with two negative axes

			return ( this.max.x < this.min.x ) || ( this.max.y < this.min.y ) || ( this.max.z < this.min.z );

		}

		/**
		 * Returns the center point of this box.
		 *
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The center point.
		 */
		getCenter( target ) {

			return this.isEmpty() ? target.set( 0, 0, 0 ) : target.addVectors( this.min, this.max ).multiplyScalar( 0.5 );

		}

		/**
		 * Returns the dimensions of this box.
		 *
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The size.
		 */
		getSize( target ) {

			return this.isEmpty() ? target.set( 0, 0, 0 ) : target.subVectors( this.max, this.min );

		}

		/**
		 * Expands the boundaries of this box to include the given point.
		 *
		 * @param {Vector3} point - The point that should be included by the bounding box.
		 * @return {Box3} A reference to this bounding box.
		 */
		expandByPoint( point ) {

			this.min.min( point );
			this.max.max( point );

			return this;

		}

		/**
		 * Expands this box equilaterally by the given vector. The width of this
		 * box will be expanded by the x component of the vector in both
		 * directions. The height of this box will be expanded by the y component of
		 * the vector in both directions. The depth of this box will be
		 * expanded by the z component of the vector in both directions.
		 *
		 * @param {Vector3} vector - The vector that should expand the bounding box.
		 * @return {Box3} A reference to this bounding box.
		 */
		expandByVector( vector ) {

			this.min.sub( vector );
			this.max.add( vector );

			return this;

		}

		/**
		 * Expands each dimension of the box by the given scalar. If negative, the
		 * dimensions of the box will be contracted.
		 *
		 * @param {number} scalar - The scalar value that should expand the bounding box.
		 * @return {Box3} A reference to this bounding box.
		 */
		expandByScalar( scalar ) {

			this.min.addScalar( - scalar );
			this.max.addScalar( scalar );

			return this;

		}

		/**
		 * Expands the boundaries of this box to include the given 3D object and
		 * its children, accounting for the object's, and children's, world
		 * transforms. The function may result in a larger box than strictly
		 * necessary (unless the precise parameter is set to true).
		 *
		 * @param {Object3D} object - The 3D object that should expand the bounding box.
		 * @param {boolean} precise - If set to `true`, the method expands the bounding box
		 * as little as necessary at the expense of more computation.
		 * @return {Box3} A reference to this bounding box.
		 */
		expandByObject( object, precise = false ) {

			// Computes the world-axis-aligned bounding box of an object (including its children),
			// accounting for both the object's, and children's, world transforms

			object.updateWorldMatrix( false, false );

			const geometry = object.geometry;

			if ( geometry !== undefined ) {

				const positionAttribute = geometry.getAttribute( 'position' );

				// precise AABB computation based on vertex data requires at least a position attribute.
				// instancing isn't supported so far and uses the normal (conservative) code path.

				if ( precise === true && positionAttribute !== undefined && object.isInstancedMesh !== true ) {

					for ( let i = 0, l = positionAttribute.count; i < l; i ++ ) {

						if ( object.isMesh === true ) {

							object.getVertexPosition( i, _vector$3 );

						} else {

							_vector$3.fromBufferAttribute( positionAttribute, i );

						}

						_vector$3.applyMatrix4( object.matrixWorld );
						this.expandByPoint( _vector$3 );

					}

				} else {

					if ( object.boundingBox !== undefined ) {

						// object-level bounding box

						if ( object.boundingBox === null ) {

							object.computeBoundingBox();

						}

						_box$2.copy( object.boundingBox );


					} else {

						// geometry-level bounding box

						if ( geometry.boundingBox === null ) {

							geometry.computeBoundingBox();

						}

						_box$2.copy( geometry.boundingBox );

					}

					_box$2.applyMatrix4( object.matrixWorld );

					this.union( _box$2 );

				}

			}

			const children = object.children;

			for ( let i = 0, l = children.length; i < l; i ++ ) {

				this.expandByObject( children[ i ], precise );

			}

			return this;

		}

		/**
		 * Returns `true` if the given point lies within or on the boundaries of this box.
		 *
		 * @param {Vector3} point - The point to test.
		 * @return {boolean} Whether the bounding box contains the given point or not.
		 */
		containsPoint( point ) {

			return point.x >= this.min.x && point.x <= this.max.x &&
				point.y >= this.min.y && point.y <= this.max.y &&
				point.z >= this.min.z && point.z <= this.max.z;

		}

		/**
		 * Returns `true` if this bounding box includes the entirety of the given bounding box.
		 * If this box and the given one are identical, this function also returns `true`.
		 *
		 * @param {Box3} box - The bounding box to test.
		 * @return {boolean} Whether the bounding box contains the given bounding box or not.
		 */
		containsBox( box ) {

			return this.min.x <= box.min.x && box.max.x <= this.max.x &&
				this.min.y <= box.min.y && box.max.y <= this.max.y &&
				this.min.z <= box.min.z && box.max.z <= this.max.z;

		}

		/**
		 * Returns a point as a proportion of this box's width, height and depth.
		 *
		 * @param {Vector3} point - A point in 3D space.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} A point as a proportion of this box's width, height and depth.
		 */
		getParameter( point, target ) {

			// This can potentially have a divide by zero if the box
			// has a size dimension of 0.

			return target.set(
				( point.x - this.min.x ) / ( this.max.x - this.min.x ),
				( point.y - this.min.y ) / ( this.max.y - this.min.y ),
				( point.z - this.min.z ) / ( this.max.z - this.min.z )
			);

		}

		/**
		 * Returns `true` if the given bounding box intersects with this bounding box.
		 *
		 * @param {Box3} box - The bounding box to test.
		 * @return {boolean} Whether the given bounding box intersects with this bounding box.
		 */
		intersectsBox( box ) {

			// using 6 splitting planes to rule out intersections.
			return box.max.x >= this.min.x && box.min.x <= this.max.x &&
				box.max.y >= this.min.y && box.min.y <= this.max.y &&
				box.max.z >= this.min.z && box.min.z <= this.max.z;

		}

		/**
		 * Returns `true` if the given bounding sphere intersects with this bounding box.
		 *
		 * @param {Sphere} sphere - The bounding sphere to test.
		 * @return {boolean} Whether the given bounding sphere intersects with this bounding box.
		 */
		intersectsSphere( sphere ) {

			// Find the point on the AABB closest to the sphere center.
			this.clampPoint( sphere.center, _vector$3 );

			// If that point is inside the sphere, the AABB and sphere intersect.
			return _vector$3.distanceToSquared( sphere.center ) <= ( sphere.radius * sphere.radius );

		}

		/**
		 * Returns `true` if the given plane intersects with this bounding box.
		 *
		 * @param {Plane} plane - The plane to test.
		 * @return {boolean} Whether the given plane intersects with this bounding box.
		 */
		intersectsPlane( plane ) {

			// We compute the minimum and maximum dot product values. If those values
			// are on the same side (back or front) of the plane, then there is no intersection.

			let min, max;

			if ( plane.normal.x > 0 ) {

				min = plane.normal.x * this.min.x;
				max = plane.normal.x * this.max.x;

			} else {

				min = plane.normal.x * this.max.x;
				max = plane.normal.x * this.min.x;

			}

			if ( plane.normal.y > 0 ) {

				min += plane.normal.y * this.min.y;
				max += plane.normal.y * this.max.y;

			} else {

				min += plane.normal.y * this.max.y;
				max += plane.normal.y * this.min.y;

			}

			if ( plane.normal.z > 0 ) {

				min += plane.normal.z * this.min.z;
				max += plane.normal.z * this.max.z;

			} else {

				min += plane.normal.z * this.max.z;
				max += plane.normal.z * this.min.z;

			}

			return ( min <= - plane.constant && max >= - plane.constant );

		}

		/**
		 * Returns `true` if the given triangle intersects with this bounding box.
		 *
		 * @param {Triangle} triangle - The triangle to test.
		 * @return {boolean} Whether the given triangle intersects with this bounding box.
		 */
		intersectsTriangle( triangle ) {

			if ( this.isEmpty() ) {

				return false;

			}

			// compute box center and extents
			this.getCenter( _center );
			_extents.subVectors( this.max, _center );

			// translate triangle to aabb origin
			_v0$1.subVectors( triangle.a, _center );
			_v1$3.subVectors( triangle.b, _center );
			_v2$2.subVectors( triangle.c, _center );

			// compute edge vectors for triangle
			_f0.subVectors( _v1$3, _v0$1 );
			_f1.subVectors( _v2$2, _v1$3 );
			_f2.subVectors( _v0$1, _v2$2 );

			// test against axes that are given by cross product combinations of the edges of the triangle and the edges of the aabb
			// make an axis testing of each of the 3 sides of the aabb against each of the 3 sides of the triangle = 9 axis of separation
			// axis_ij = u_i x f_j (u0, u1, u2 = face normals of aabb = x,y,z axes vectors since aabb is axis aligned)
			let axes = [
				0, - _f0.z, _f0.y, 0, - _f1.z, _f1.y, 0, - _f2.z, _f2.y,
				_f0.z, 0, - _f0.x, _f1.z, 0, - _f1.x, _f2.z, 0, - _f2.x,
				- _f0.y, _f0.x, 0, - _f1.y, _f1.x, 0, - _f2.y, _f2.x, 0
			];
			if ( ! satForAxes( axes, _v0$1, _v1$3, _v2$2, _extents ) ) {

				return false;

			}

			// test 3 face normals from the aabb
			axes = [ 1, 0, 0, 0, 1, 0, 0, 0, 1 ];
			if ( ! satForAxes( axes, _v0$1, _v1$3, _v2$2, _extents ) ) {

				return false;

			}

			// finally testing the face normal of the triangle
			// use already existing triangle edge vectors here
			_triangleNormal.crossVectors( _f0, _f1 );
			axes = [ _triangleNormal.x, _triangleNormal.y, _triangleNormal.z ];

			return satForAxes( axes, _v0$1, _v1$3, _v2$2, _extents );

		}

		/**
		 * Clamps the given point within the bounds of this box.
		 *
		 * @param {Vector3} point - The point to clamp.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The clamped point.
		 */
		clampPoint( point, target ) {

			return target.copy( point ).clamp( this.min, this.max );

		}

		/**
		 * Returns the euclidean distance from any edge of this box to the specified point. If
		 * the given point lies inside of this box, the distance will be `0`.
		 *
		 * @param {Vector3} point - The point to compute the distance to.
		 * @return {number} The euclidean distance.
		 */
		distanceToPoint( point ) {

			return this.clampPoint( point, _vector$3 ).distanceTo( point );

		}

		/**
		 * Returns a bounding sphere that encloses this bounding box.
		 *
		 * @param {Sphere} target - The target sphere that is used to store the method's result.
		 * @return {Sphere} The bounding sphere that encloses this bounding box.
		 */
		getBoundingSphere( target ) {

			if ( this.isEmpty() ) {

				target.makeEmpty();

			} else {

				this.getCenter( target.center );

				target.radius = this.getSize( _vector$3 ).length() * 0.5;

			}

			return target;

		}

		/**
		 * Computes the intersection of this bounding box and the given one, setting the upper
		 * bound of this box to the lesser of the two boxes' upper bounds and the
		 * lower bound of this box to the greater of the two boxes' lower bounds. If
		 * there's no overlap, makes this box empty.
		 *
		 * @param {Box3} box - The bounding box to intersect with.
		 * @return {Box3} A reference to this bounding box.
		 */
		intersect( box ) {

			this.min.max( box.min );
			this.max.min( box.max );

			// ensure that if there is no overlap, the result is fully empty, not slightly empty with non-inf/+inf values that will cause subsequence intersects to erroneously return valid values.
			if ( this.isEmpty() ) this.makeEmpty();

			return this;

		}

		/**
		 * Computes the union of this box and another and the given one, setting the upper
		 * bound of this box to the greater of the two boxes' upper bounds and the
		 * lower bound of this box to the lesser of the two boxes' lower bounds.
		 *
		 * @param {Box3} box - The bounding box that will be unioned with this instance.
		 * @return {Box3} A reference to this bounding box.
		 */
		union( box ) {

			this.min.min( box.min );
			this.max.max( box.max );

			return this;

		}

		/**
		 * Transforms this bounding box by the given 4x4 transformation matrix.
		 *
		 * @param {Matrix4} matrix - The transformation matrix.
		 * @return {Box3} A reference to this bounding box.
		 */
		applyMatrix4( matrix ) {

			// transform of empty box is an empty box.
			if ( this.isEmpty() ) return this;

			// NOTE: I am using a binary pattern to specify all 2^3 combinations below
			_points[ 0 ].set( this.min.x, this.min.y, this.min.z ).applyMatrix4( matrix ); // 000
			_points[ 1 ].set( this.min.x, this.min.y, this.max.z ).applyMatrix4( matrix ); // 001
			_points[ 2 ].set( this.min.x, this.max.y, this.min.z ).applyMatrix4( matrix ); // 010
			_points[ 3 ].set( this.min.x, this.max.y, this.max.z ).applyMatrix4( matrix ); // 011
			_points[ 4 ].set( this.max.x, this.min.y, this.min.z ).applyMatrix4( matrix ); // 100
			_points[ 5 ].set( this.max.x, this.min.y, this.max.z ).applyMatrix4( matrix ); // 101
			_points[ 6 ].set( this.max.x, this.max.y, this.min.z ).applyMatrix4( matrix ); // 110
			_points[ 7 ].set( this.max.x, this.max.y, this.max.z ).applyMatrix4( matrix ); // 111

			this.setFromPoints( _points );

			return this;

		}

		/**
		 * Adds the given offset to both the upper and lower bounds of this bounding box,
		 * effectively moving it in 3D space.
		 *
		 * @param {Vector3} offset - The offset that should be used to translate the bounding box.
		 * @return {Box3} A reference to this bounding box.
		 */
		translate( offset ) {

			this.min.add( offset );
			this.max.add( offset );

			return this;

		}

		/**
		 * Returns `true` if this bounding box is equal with the given one.
		 *
		 * @param {Box3} box - The box to test for equality.
		 * @return {boolean} Whether this bounding box is equal with the given one.
		 */
		equals( box ) {

			return box.min.equals( this.min ) && box.max.equals( this.max );

		}

		/**
		 * Returns a serialized structure of the bounding box.
		 *
		 * @return {Object} Serialized structure with fields representing the object state.
		 */
		toJSON() {

			return {
				min: this.min.toArray(),
				max: this.max.toArray()
			};

		}

		/**
		 * Returns a serialized structure of the bounding box.
		 *
		 * @param {Object} json - The serialized json to set the box from.
		 * @return {Box3} A reference to this bounding box.
		 */
		fromJSON( json ) {

			this.min.fromArray( json.min );
			this.max.fromArray( json.max );
			return this;

		}

	}

	const _points = [
		/*@__PURE__*/ new Vector3(),
		/*@__PURE__*/ new Vector3(),
		/*@__PURE__*/ new Vector3(),
		/*@__PURE__*/ new Vector3(),
		/*@__PURE__*/ new Vector3(),
		/*@__PURE__*/ new Vector3(),
		/*@__PURE__*/ new Vector3(),
		/*@__PURE__*/ new Vector3()
	];

	const _vector$3 = /*@__PURE__*/ new Vector3();

	const _box$2 = /*@__PURE__*/ new Box3();

	// triangle centered vertices

	const _v0$1 = /*@__PURE__*/ new Vector3();
	const _v1$3 = /*@__PURE__*/ new Vector3();
	const _v2$2 = /*@__PURE__*/ new Vector3();

	// triangle edge vectors

	const _f0 = /*@__PURE__*/ new Vector3();
	const _f1 = /*@__PURE__*/ new Vector3();
	const _f2 = /*@__PURE__*/ new Vector3();

	const _center = /*@__PURE__*/ new Vector3();
	const _extents = /*@__PURE__*/ new Vector3();
	const _triangleNormal = /*@__PURE__*/ new Vector3();
	const _testAxis = /*@__PURE__*/ new Vector3();

	function satForAxes( axes, v0, v1, v2, extents ) {

		for ( let i = 0, j = axes.length - 3; i <= j; i += 3 ) {

			_testAxis.fromArray( axes, i );
			// project the aabb onto the separating axis
			const r = extents.x * Math.abs( _testAxis.x ) + extents.y * Math.abs( _testAxis.y ) + extents.z * Math.abs( _testAxis.z );
			// project all 3 vertices of the triangle onto the separating axis
			const p0 = v0.dot( _testAxis );
			const p1 = v1.dot( _testAxis );
			const p2 = v2.dot( _testAxis );
			// actual test, basically see if either of the most extreme of the triangle points intersects r
			if ( Math.max( - Math.max( p0, p1, p2 ), Math.min( p0, p1, p2 ) ) > r ) {

				// points of the projected triangle are outside the projected half-length of the aabb
				// the axis is separating and we can exit
				return false;

			}

		}

		return true;

	}

	/**
	 * Represents a 3x3 matrix.
	 *
	 * A Note on Row-Major and Column-Major Ordering:
	 *
	 * The constructor and {@link Matrix3#set} method take arguments in
	 * [row-major]{@link https://en.wikipedia.org/wiki/Row-_and_column-major_order#Column-major_order}
	 * order, while internally they are stored in the {@link Matrix3#elements} array in column-major order.
	 * This means that calling:
	 * ```js
	 * const m = new THREE.Matrix();
	 * m.set( 11, 12, 13,
	 *        21, 22, 23,
	 *        31, 32, 33 );
	 * ```
	 * will result in the elements array containing:
	 * ```js
	 * m.elements = [ 11, 21, 31,
	 *                12, 22, 32,
	 *                13, 23, 33 ];
	 * ```
	 * and internally all calculations are performed using column-major ordering.
	 * However, as the actual ordering makes no difference mathematically and
	 * most people are used to thinking about matrices in row-major order, the
	 * three.js documentation shows matrices in row-major order. Just bear in
	 * mind that if you are reading the source code, you'll have to take the
	 * transpose of any matrices outlined here to make sense of the calculations.
	 */
	class Matrix3 {

		/**
		 * Constructs a new 3x3 matrix. The arguments are supposed to be
		 * in row-major order. If no arguments are provided, the constructor
		 * initializes the matrix as an identity matrix.
		 *
		 * @param {number} [n11] - 1-1 matrix element.
		 * @param {number} [n12] - 1-2 matrix element.
		 * @param {number} [n13] - 1-3 matrix element.
		 * @param {number} [n21] - 2-1 matrix element.
		 * @param {number} [n22] - 2-2 matrix element.
		 * @param {number} [n23] - 2-3 matrix element.
		 * @param {number} [n31] - 3-1 matrix element.
		 * @param {number} [n32] - 3-2 matrix element.
		 * @param {number} [n33] - 3-3 matrix element.
		 */
		constructor( n11, n12, n13, n21, n22, n23, n31, n32, n33 ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			Matrix3.prototype.isMatrix3 = true;

			/**
			 * A column-major list of matrix values.
			 *
			 * @type {Array<number>}
			 */
			this.elements = [

				1, 0, 0,
				0, 1, 0,
				0, 0, 1

			];

			if ( n11 !== undefined ) {

				this.set( n11, n12, n13, n21, n22, n23, n31, n32, n33 );

			}

		}

		/**
		 * Sets the elements of the matrix.The arguments are supposed to be
		 * in row-major order.
		 *
		 * @param {number} [n11] - 1-1 matrix element.
		 * @param {number} [n12] - 1-2 matrix element.
		 * @param {number} [n13] - 1-3 matrix element.
		 * @param {number} [n21] - 2-1 matrix element.
		 * @param {number} [n22] - 2-2 matrix element.
		 * @param {number} [n23] - 2-3 matrix element.
		 * @param {number} [n31] - 3-1 matrix element.
		 * @param {number} [n32] - 3-2 matrix element.
		 * @param {number} [n33] - 3-3 matrix element.
		 * @return {Matrix3} A reference to this matrix.
		 */
		set( n11, n12, n13, n21, n22, n23, n31, n32, n33 ) {

			const te = this.elements;

			te[ 0 ] = n11; te[ 1 ] = n21; te[ 2 ] = n31;
			te[ 3 ] = n12; te[ 4 ] = n22; te[ 5 ] = n32;
			te[ 6 ] = n13; te[ 7 ] = n23; te[ 8 ] = n33;

			return this;

		}

		/**
		 * Sets this matrix to the 3x3 identity matrix.
		 *
		 * @return {Matrix3} A reference to this matrix.
		 */
		identity() {

			this.set(

				1, 0, 0,
				0, 1, 0,
				0, 0, 1

			);

			return this;

		}

		/**
		 * Copies the values of the given matrix to this instance.
		 *
		 * @param {Matrix3} m - The matrix to copy.
		 * @return {Matrix3} A reference to this matrix.
		 */
		copy( m ) {

			const te = this.elements;
			const me = m.elements;

			te[ 0 ] = me[ 0 ]; te[ 1 ] = me[ 1 ]; te[ 2 ] = me[ 2 ];
			te[ 3 ] = me[ 3 ]; te[ 4 ] = me[ 4 ]; te[ 5 ] = me[ 5 ];
			te[ 6 ] = me[ 6 ]; te[ 7 ] = me[ 7 ]; te[ 8 ] = me[ 8 ];

			return this;

		}

		/**
		 * Extracts the basis of this matrix into the three axis vectors provided.
		 *
		 * @param {Vector3} xAxis - The basis's x axis.
		 * @param {Vector3} yAxis - The basis's y axis.
		 * @param {Vector3} zAxis - The basis's z axis.
		 * @return {Matrix3} A reference to this matrix.
		 */
		extractBasis( xAxis, yAxis, zAxis ) {

			xAxis.setFromMatrix3Column( this, 0 );
			yAxis.setFromMatrix3Column( this, 1 );
			zAxis.setFromMatrix3Column( this, 2 );

			return this;

		}

		/**
		 * Set this matrix to the upper 3x3 matrix of the given 4x4 matrix.
		 *
		 * @param {Matrix4} m - The 4x4 matrix.
		 * @return {Matrix3} A reference to this matrix.
		 */
		setFromMatrix4( m ) {

			const me = m.elements;

			this.set(

				me[ 0 ], me[ 4 ], me[ 8 ],
				me[ 1 ], me[ 5 ], me[ 9 ],
				me[ 2 ], me[ 6 ], me[ 10 ]

			);

			return this;

		}

		/**
		 * Post-multiplies this matrix by the given 3x3 matrix.
		 *
		 * @param {Matrix3} m - The matrix to multiply with.
		 * @return {Matrix3} A reference to this matrix.
		 */
		multiply( m ) {

			return this.multiplyMatrices( this, m );

		}

		/**
		 * Pre-multiplies this matrix by the given 3x3 matrix.
		 *
		 * @param {Matrix3} m - The matrix to multiply with.
		 * @return {Matrix3} A reference to this matrix.
		 */
		premultiply( m ) {

			return this.multiplyMatrices( m, this );

		}

		/**
		 * Multiples the given 3x3 matrices and stores the result
		 * in this matrix.
		 *
		 * @param {Matrix3} a - The first matrix.
		 * @param {Matrix3} b - The second matrix.
		 * @return {Matrix3} A reference to this matrix.
		 */
		multiplyMatrices( a, b ) {

			const ae = a.elements;
			const be = b.elements;
			const te = this.elements;

			const a11 = ae[ 0 ], a12 = ae[ 3 ], a13 = ae[ 6 ];
			const a21 = ae[ 1 ], a22 = ae[ 4 ], a23 = ae[ 7 ];
			const a31 = ae[ 2 ], a32 = ae[ 5 ], a33 = ae[ 8 ];

			const b11 = be[ 0 ], b12 = be[ 3 ], b13 = be[ 6 ];
			const b21 = be[ 1 ], b22 = be[ 4 ], b23 = be[ 7 ];
			const b31 = be[ 2 ], b32 = be[ 5 ], b33 = be[ 8 ];

			te[ 0 ] = a11 * b11 + a12 * b21 + a13 * b31;
			te[ 3 ] = a11 * b12 + a12 * b22 + a13 * b32;
			te[ 6 ] = a11 * b13 + a12 * b23 + a13 * b33;

			te[ 1 ] = a21 * b11 + a22 * b21 + a23 * b31;
			te[ 4 ] = a21 * b12 + a22 * b22 + a23 * b32;
			te[ 7 ] = a21 * b13 + a22 * b23 + a23 * b33;

			te[ 2 ] = a31 * b11 + a32 * b21 + a33 * b31;
			te[ 5 ] = a31 * b12 + a32 * b22 + a33 * b32;
			te[ 8 ] = a31 * b13 + a32 * b23 + a33 * b33;

			return this;

		}

		/**
		 * Multiplies every component of the matrix by the given scalar.
		 *
		 * @param {number} s - The scalar.
		 * @return {Matrix3} A reference to this matrix.
		 */
		multiplyScalar( s ) {

			const te = this.elements;

			te[ 0 ] *= s; te[ 3 ] *= s; te[ 6 ] *= s;
			te[ 1 ] *= s; te[ 4 ] *= s; te[ 7 ] *= s;
			te[ 2 ] *= s; te[ 5 ] *= s; te[ 8 ] *= s;

			return this;

		}

		/**
		 * Computes and returns the determinant of this matrix.
		 *
		 * @return {number} The determinant.
		 */
		determinant() {

			const te = this.elements;

			const a = te[ 0 ], b = te[ 1 ], c = te[ 2 ],
				d = te[ 3 ], e = te[ 4 ], f = te[ 5 ],
				g = te[ 6 ], h = te[ 7 ], i = te[ 8 ];

			return a * e * i - a * f * h - b * d * i + b * f * g + c * d * h - c * e * g;

		}

		/**
		 * Inverts this matrix, using the [analytic method]{@link https://en.wikipedia.org/wiki/Invertible_matrix#Analytic_solution}.
		 * You can not invert with a determinant of zero. If you attempt this, the method produces
		 * a zero matrix instead.
		 *
		 * @return {Matrix3} A reference to this matrix.
		 */
		invert() {

			const te = this.elements,

				n11 = te[ 0 ], n21 = te[ 1 ], n31 = te[ 2 ],
				n12 = te[ 3 ], n22 = te[ 4 ], n32 = te[ 5 ],
				n13 = te[ 6 ], n23 = te[ 7 ], n33 = te[ 8 ],

				t11 = n33 * n22 - n32 * n23,
				t12 = n32 * n13 - n33 * n12,
				t13 = n23 * n12 - n22 * n13,

				det = n11 * t11 + n21 * t12 + n31 * t13;

			if ( det === 0 ) return this.set( 0, 0, 0, 0, 0, 0, 0, 0, 0 );

			const detInv = 1 / det;

			te[ 0 ] = t11 * detInv;
			te[ 1 ] = ( n31 * n23 - n33 * n21 ) * detInv;
			te[ 2 ] = ( n32 * n21 - n31 * n22 ) * detInv;

			te[ 3 ] = t12 * detInv;
			te[ 4 ] = ( n33 * n11 - n31 * n13 ) * detInv;
			te[ 5 ] = ( n31 * n12 - n32 * n11 ) * detInv;

			te[ 6 ] = t13 * detInv;
			te[ 7 ] = ( n21 * n13 - n23 * n11 ) * detInv;
			te[ 8 ] = ( n22 * n11 - n21 * n12 ) * detInv;

			return this;

		}

		/**
		 * Transposes this matrix in place.
		 *
		 * @return {Matrix3} A reference to this matrix.
		 */
		transpose() {

			let tmp;
			const m = this.elements;

			tmp = m[ 1 ]; m[ 1 ] = m[ 3 ]; m[ 3 ] = tmp;
			tmp = m[ 2 ]; m[ 2 ] = m[ 6 ]; m[ 6 ] = tmp;
			tmp = m[ 5 ]; m[ 5 ] = m[ 7 ]; m[ 7 ] = tmp;

			return this;

		}

		/**
		 * Computes the normal matrix which is the inverse transpose of the upper
		 * left 3x3 portion of the given 4x4 matrix.
		 *
		 * @param {Matrix4} matrix4 - The 4x4 matrix.
		 * @return {Matrix3} A reference to this matrix.
		 */
		getNormalMatrix( matrix4 ) {

			return this.setFromMatrix4( matrix4 ).invert().transpose();

		}

		/**
		 * Transposes this matrix into the supplied array, and returns itself unchanged.
		 *
		 * @param {Array<number>} r - An array to store the transposed matrix elements.
		 * @return {Matrix3} A reference to this matrix.
		 */
		transposeIntoArray( r ) {

			const m = this.elements;

			r[ 0 ] = m[ 0 ];
			r[ 1 ] = m[ 3 ];
			r[ 2 ] = m[ 6 ];
			r[ 3 ] = m[ 1 ];
			r[ 4 ] = m[ 4 ];
			r[ 5 ] = m[ 7 ];
			r[ 6 ] = m[ 2 ];
			r[ 7 ] = m[ 5 ];
			r[ 8 ] = m[ 8 ];

			return this;

		}

		/**
		 * Sets the UV transform matrix from offset, repeat, rotation, and center.
		 *
		 * @param {number} tx - Offset x.
		 * @param {number} ty - Offset y.
		 * @param {number} sx - Repeat x.
		 * @param {number} sy - Repeat y.
		 * @param {number} rotation - Rotation, in radians. Positive values rotate counterclockwise.
		 * @param {number} cx - Center x of rotation.
		 * @param {number} cy - Center y of rotation
		 * @return {Matrix3} A reference to this matrix.
		 */
		setUvTransform( tx, ty, sx, sy, rotation, cx, cy ) {

			const c = Math.cos( rotation );
			const s = Math.sin( rotation );

			this.set(
				sx * c, sx * s, - sx * ( c * cx + s * cy ) + cx + tx,
				- sy * s, sy * c, - sy * ( - s * cx + c * cy ) + cy + ty,
				0, 0, 1
			);

			return this;

		}

		/**
		 * Scales this matrix with the given scalar values.
		 *
		 * @param {number} sx - The amount to scale in the X axis.
		 * @param {number} sy - The amount to scale in the Y axis.
		 * @return {Matrix3} A reference to this matrix.
		 */
		scale( sx, sy ) {

			this.premultiply( _m3.makeScale( sx, sy ) );

			return this;

		}

		/**
		 * Rotates this matrix by the given angle.
		 *
		 * @param {number} theta - The rotation in radians.
		 * @return {Matrix3} A reference to this matrix.
		 */
		rotate( theta ) {

			this.premultiply( _m3.makeRotation( - theta ) );

			return this;

		}

		/**
		 * Translates this matrix by the given scalar values.
		 *
		 * @param {number} tx - The amount to translate in the X axis.
		 * @param {number} ty - The amount to translate in the Y axis.
		 * @return {Matrix3} A reference to this matrix.
		 */
		translate( tx, ty ) {

			this.premultiply( _m3.makeTranslation( tx, ty ) );

			return this;

		}

		// for 2D Transforms

		/**
		 * Sets this matrix as a 2D translation transform.
		 *
		 * @param {number|Vector2} x - The amount to translate in the X axis or alternatively a translation vector.
		 * @param {number} y - The amount to translate in the Y axis.
		 * @return {Matrix3} A reference to this matrix.
		 */
		makeTranslation( x, y ) {

			if ( x.isVector2 ) {

				this.set(

					1, 0, x.x,
					0, 1, x.y,
					0, 0, 1

				);

			} else {

				this.set(

					1, 0, x,
					0, 1, y,
					0, 0, 1

				);

			}

			return this;

		}

		/**
		 * Sets this matrix as a 2D rotational transformation.
		 *
		 * @param {number} theta - The rotation in radians.
		 * @return {Matrix3} A reference to this matrix.
		 */
		makeRotation( theta ) {

			// counterclockwise

			const c = Math.cos( theta );
			const s = Math.sin( theta );

			this.set(

				c, - s, 0,
				s, c, 0,
				0, 0, 1

			);

			return this;

		}

		/**
		 * Sets this matrix as a 2D scale transform.
		 *
		 * @param {number} x - The amount to scale in the X axis.
		 * @param {number} y - The amount to scale in the Y axis.
		 * @return {Matrix3} A reference to this matrix.
		 */
		makeScale( x, y ) {

			this.set(

				x, 0, 0,
				0, y, 0,
				0, 0, 1

			);

			return this;

		}

		/**
		 * Returns `true` if this matrix is equal with the given one.
		 *
		 * @param {Matrix3} matrix - The matrix to test for equality.
		 * @return {boolean} Whether this matrix is equal with the given one.
		 */
		equals( matrix ) {

			const te = this.elements;
			const me = matrix.elements;

			for ( let i = 0; i < 9; i ++ ) {

				if ( te[ i ] !== me[ i ] ) return false;

			}

			return true;

		}

		/**
		 * Sets the elements of the matrix from the given array.
		 *
		 * @param {Array<number>} array - The matrix elements in column-major order.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Matrix3} A reference to this matrix.
		 */
		fromArray( array, offset = 0 ) {

			for ( let i = 0; i < 9; i ++ ) {

				this.elements[ i ] = array[ i + offset ];

			}

			return this;

		}

		/**
		 * Writes the elements of this matrix to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number>} [array=[]] - The target array holding the matrix elements in column-major order.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number>} The matrix elements in column-major order.
		 */
		toArray( array = [], offset = 0 ) {

			const te = this.elements;

			array[ offset ] = te[ 0 ];
			array[ offset + 1 ] = te[ 1 ];
			array[ offset + 2 ] = te[ 2 ];

			array[ offset + 3 ] = te[ 3 ];
			array[ offset + 4 ] = te[ 4 ];
			array[ offset + 5 ] = te[ 5 ];

			array[ offset + 6 ] = te[ 6 ];
			array[ offset + 7 ] = te[ 7 ];
			array[ offset + 8 ] = te[ 8 ];

			return array;

		}

		/**
		 * Returns a matrix with copied values from this instance.
		 *
		 * @return {Matrix3} A clone of this instance.
		 */
		clone() {

			return new this.constructor().fromArray( this.elements );

		}

	}

	const _m3 = /*@__PURE__*/ new Matrix3();

	function arrayNeedsUint32( array ) {

		// assumes larger values usually on last

		for ( let i = array.length - 1; i >= 0; -- i ) {

			if ( array[ i ] >= 65535 ) return true; // account for PRIMITIVE_RESTART_FIXED_INDEX, #24565

		}

		return false;

	}

	const _cache = {};

	function warnOnce( message ) {

		if ( message in _cache ) return;

		_cache[ message ] = true;

		console.warn( message );

	}

	const LINEAR_REC709_TO_XYZ = /*@__PURE__*/ new Matrix3().set(
		0.4123908, 0.3575843, 0.1804808,
		0.2126390, 0.7151687, 0.0721923,
		0.0193308, 0.1191948, 0.9505322
	);

	const XYZ_TO_LINEAR_REC709 = /*@__PURE__*/ new Matrix3().set(
		3.2409699, - 1.5373832, - 0.4986108,
		- 0.9692436, 1.8759675, 0.0415551,
		0.0556301, - 0.2039770, 1.0569715
	);

	function createColorManagement() {

		const ColorManagement = {

			enabled: true,

			workingColorSpace: LinearSRGBColorSpace,

			/**
			 * Implementations of supported color spaces.
			 *
			 * Required:
			 *	- primaries: chromaticity coordinates [ rx ry gx gy bx by ]
			 *	- whitePoint: reference white [ x y ]
			 *	- transfer: transfer function (pre-defined)
			 *	- toXYZ: Matrix3 RGB to XYZ transform
			 *	- fromXYZ: Matrix3 XYZ to RGB transform
			 *	- luminanceCoefficients: RGB luminance coefficients
			 *
			 * Optional:
			 *  - outputColorSpaceConfig: { drawingBufferColorSpace: ColorSpace, toneMappingMode: 'extended' | 'standard' }
			 *  - workingColorSpaceConfig: { unpackColorSpace: ColorSpace }
			 *
			 * Reference:
			 * - https://www.russellcottrell.com/photo/matrixCalculator.htm
			 */
			spaces: {},

			convert: function ( color, sourceColorSpace, targetColorSpace ) {

				if ( this.enabled === false || sourceColorSpace === targetColorSpace || ! sourceColorSpace || ! targetColorSpace ) {

					return color;

				}

				if ( this.spaces[ sourceColorSpace ].transfer === SRGBTransfer ) {

					color.r = SRGBToLinear( color.r );
					color.g = SRGBToLinear( color.g );
					color.b = SRGBToLinear( color.b );

				}

				if ( this.spaces[ sourceColorSpace ].primaries !== this.spaces[ targetColorSpace ].primaries ) {

					color.applyMatrix3( this.spaces[ sourceColorSpace ].toXYZ );
					color.applyMatrix3( this.spaces[ targetColorSpace ].fromXYZ );

				}

				if ( this.spaces[ targetColorSpace ].transfer === SRGBTransfer ) {

					color.r = LinearToSRGB( color.r );
					color.g = LinearToSRGB( color.g );
					color.b = LinearToSRGB( color.b );

				}

				return color;

			},

			workingToColorSpace: function ( color, targetColorSpace ) {

				return this.convert( color, this.workingColorSpace, targetColorSpace );

			},

			colorSpaceToWorking: function ( color, sourceColorSpace ) {

				return this.convert( color, sourceColorSpace, this.workingColorSpace );

			},

			getPrimaries: function ( colorSpace ) {

				return this.spaces[ colorSpace ].primaries;

			},

			getTransfer: function ( colorSpace ) {

				if ( colorSpace === NoColorSpace ) return LinearTransfer;

				return this.spaces[ colorSpace ].transfer;

			},

			getToneMappingMode: function ( colorSpace ) {

				return this.spaces[ colorSpace ].outputColorSpaceConfig.toneMappingMode || 'standard';

			},

			getLuminanceCoefficients: function ( target, colorSpace = this.workingColorSpace ) {

				return target.fromArray( this.spaces[ colorSpace ].luminanceCoefficients );

			},

			define: function ( colorSpaces ) {

				Object.assign( this.spaces, colorSpaces );

			},

			// Internal APIs

			_getMatrix: function ( targetMatrix, sourceColorSpace, targetColorSpace ) {

				return targetMatrix
					.copy( this.spaces[ sourceColorSpace ].toXYZ )
					.multiply( this.spaces[ targetColorSpace ].fromXYZ );

			},

			_getDrawingBufferColorSpace: function ( colorSpace ) {

				return this.spaces[ colorSpace ].outputColorSpaceConfig.drawingBufferColorSpace;

			},

			_getUnpackColorSpace: function ( colorSpace = this.workingColorSpace ) {

				return this.spaces[ colorSpace ].workingColorSpaceConfig.unpackColorSpace;

			},

			// Deprecated

			fromWorkingColorSpace: function ( color, targetColorSpace ) {

				warnOnce( 'THREE.ColorManagement: .fromWorkingColorSpace() has been renamed to .workingToColorSpace().' ); // @deprecated, r177

				return ColorManagement.workingToColorSpace( color, targetColorSpace );

			},

			toWorkingColorSpace: function ( color, sourceColorSpace ) {

				warnOnce( 'THREE.ColorManagement: .toWorkingColorSpace() has been renamed to .colorSpaceToWorking().' ); // @deprecated, r177

				return ColorManagement.colorSpaceToWorking( color, sourceColorSpace );

			},

		};

		/******************************************************************************
		 * sRGB definitions
		 */

		const REC709_PRIMARIES = [ 0.640, 0.330, 0.300, 0.600, 0.150, 0.060 ];
		const REC709_LUMINANCE_COEFFICIENTS = [ 0.2126, 0.7152, 0.0722 ];
		const D65 = [ 0.3127, 0.3290 ];

		ColorManagement.define( {

			[ LinearSRGBColorSpace ]: {
				primaries: REC709_PRIMARIES,
				whitePoint: D65,
				transfer: LinearTransfer,
				toXYZ: LINEAR_REC709_TO_XYZ,
				fromXYZ: XYZ_TO_LINEAR_REC709,
				luminanceCoefficients: REC709_LUMINANCE_COEFFICIENTS,
				workingColorSpaceConfig: { unpackColorSpace: SRGBColorSpace },
				outputColorSpaceConfig: { drawingBufferColorSpace: SRGBColorSpace }
			},

			[ SRGBColorSpace ]: {
				primaries: REC709_PRIMARIES,
				whitePoint: D65,
				transfer: SRGBTransfer,
				toXYZ: LINEAR_REC709_TO_XYZ,
				fromXYZ: XYZ_TO_LINEAR_REC709,
				luminanceCoefficients: REC709_LUMINANCE_COEFFICIENTS,
				outputColorSpaceConfig: { drawingBufferColorSpace: SRGBColorSpace }
			},

		} );

		return ColorManagement;

	}

	const ColorManagement = /*@__PURE__*/ createColorManagement();

	function SRGBToLinear( c ) {

		return ( c < 0.04045 ) ? c * 0.0773993808 : Math.pow( c * 0.9478672986 + 0.0521327014, 2.4 );

	}

	function LinearToSRGB( c ) {

		return ( c < 0.0031308 ) ? c * 12.92 : 1.055 * ( Math.pow( c, 0.41666 ) ) - 0.055;

	}

	const _colorKeywords = { 'aliceblue': 0xF0F8FF, 'antiquewhite': 0xFAEBD7, 'aqua': 0x00FFFF, 'aquamarine': 0x7FFFD4, 'azure': 0xF0FFFF,
		'beige': 0xF5F5DC, 'bisque': 0xFFE4C4, 'black': 0x000000, 'blanchedalmond': 0xFFEBCD, 'blue': 0x0000FF, 'blueviolet': 0x8A2BE2,
		'brown': 0xA52A2A, 'burlywood': 0xDEB887, 'cadetblue': 0x5F9EA0, 'chartreuse': 0x7FFF00, 'chocolate': 0xD2691E, 'coral': 0xFF7F50,
		'cornflowerblue': 0x6495ED, 'cornsilk': 0xFFF8DC, 'crimson': 0xDC143C, 'cyan': 0x00FFFF, 'darkblue': 0x00008B, 'darkcyan': 0x008B8B,
		'darkgoldenrod': 0xB8860B, 'darkgray': 0xA9A9A9, 'darkgreen': 0x006400, 'darkgrey': 0xA9A9A9, 'darkkhaki': 0xBDB76B, 'darkmagenta': 0x8B008B,
		'darkolivegreen': 0x556B2F, 'darkorange': 0xFF8C00, 'darkorchid': 0x9932CC, 'darkred': 0x8B0000, 'darksalmon': 0xE9967A, 'darkseagreen': 0x8FBC8F,
		'darkslateblue': 0x483D8B, 'darkslategray': 0x2F4F4F, 'darkslategrey': 0x2F4F4F, 'darkturquoise': 0x00CED1, 'darkviolet': 0x9400D3,
		'deeppink': 0xFF1493, 'deepskyblue': 0x00BFFF, 'dimgray': 0x696969, 'dimgrey': 0x696969, 'dodgerblue': 0x1E90FF, 'firebrick': 0xB22222,
		'floralwhite': 0xFFFAF0, 'forestgreen': 0x228B22, 'fuchsia': 0xFF00FF, 'gainsboro': 0xDCDCDC, 'ghostwhite': 0xF8F8FF, 'gold': 0xFFD700,
		'goldenrod': 0xDAA520, 'gray': 0x808080, 'green': 0x008000, 'greenyellow': 0xADFF2F, 'grey': 0x808080, 'honeydew': 0xF0FFF0, 'hotpink': 0xFF69B4,
		'indianred': 0xCD5C5C, 'indigo': 0x4B0082, 'ivory': 0xFFFFF0, 'khaki': 0xF0E68C, 'lavender': 0xE6E6FA, 'lavenderblush': 0xFFF0F5, 'lawngreen': 0x7CFC00,
		'lemonchiffon': 0xFFFACD, 'lightblue': 0xADD8E6, 'lightcoral': 0xF08080, 'lightcyan': 0xE0FFFF, 'lightgoldenrodyellow': 0xFAFAD2, 'lightgray': 0xD3D3D3,
		'lightgreen': 0x90EE90, 'lightgrey': 0xD3D3D3, 'lightpink': 0xFFB6C1, 'lightsalmon': 0xFFA07A, 'lightseagreen': 0x20B2AA, 'lightskyblue': 0x87CEFA,
		'lightslategray': 0x778899, 'lightslategrey': 0x778899, 'lightsteelblue': 0xB0C4DE, 'lightyellow': 0xFFFFE0, 'lime': 0x00FF00, 'limegreen': 0x32CD32,
		'linen': 0xFAF0E6, 'magenta': 0xFF00FF, 'maroon': 0x800000, 'mediumaquamarine': 0x66CDAA, 'mediumblue': 0x0000CD, 'mediumorchid': 0xBA55D3,
		'mediumpurple': 0x9370DB, 'mediumseagreen': 0x3CB371, 'mediumslateblue': 0x7B68EE, 'mediumspringgreen': 0x00FA9A, 'mediumturquoise': 0x48D1CC,
		'mediumvioletred': 0xC71585, 'midnightblue': 0x191970, 'mintcream': 0xF5FFFA, 'mistyrose': 0xFFE4E1, 'moccasin': 0xFFE4B5, 'navajowhite': 0xFFDEAD,
		'navy': 0x000080, 'oldlace': 0xFDF5E6, 'olive': 0x808000, 'olivedrab': 0x6B8E23, 'orange': 0xFFA500, 'orangered': 0xFF4500, 'orchid': 0xDA70D6,
		'palegoldenrod': 0xEEE8AA, 'palegreen': 0x98FB98, 'paleturquoise': 0xAFEEEE, 'palevioletred': 0xDB7093, 'papayawhip': 0xFFEFD5, 'peachpuff': 0xFFDAB9,
		'peru': 0xCD853F, 'pink': 0xFFC0CB, 'plum': 0xDDA0DD, 'powderblue': 0xB0E0E6, 'purple': 0x800080, 'rebeccapurple': 0x663399, 'red': 0xFF0000, 'rosybrown': 0xBC8F8F,
		'royalblue': 0x4169E1, 'saddlebrown': 0x8B4513, 'salmon': 0xFA8072, 'sandybrown': 0xF4A460, 'seagreen': 0x2E8B57, 'seashell': 0xFFF5EE,
		'sienna': 0xA0522D, 'silver': 0xC0C0C0, 'skyblue': 0x87CEEB, 'slateblue': 0x6A5ACD, 'slategray': 0x708090, 'slategrey': 0x708090, 'snow': 0xFFFAFA,
		'springgreen': 0x00FF7F, 'steelblue': 0x4682B4, 'tan': 0xD2B48C, 'teal': 0x008080, 'thistle': 0xD8BFD8, 'tomato': 0xFF6347, 'turquoise': 0x40E0D0,
		'violet': 0xEE82EE, 'wheat': 0xF5DEB3, 'white': 0xFFFFFF, 'whitesmoke': 0xF5F5F5, 'yellow': 0xFFFF00, 'yellowgreen': 0x9ACD32 };

	const _hslA = { h: 0, s: 0, l: 0 };
	const _hslB = { h: 0, s: 0, l: 0 };

	function hue2rgb( p, q, t ) {

		if ( t < 0 ) t += 1;
		if ( t > 1 ) t -= 1;
		if ( t < 1 / 6 ) return p + ( q - p ) * 6 * t;
		if ( t < 1 / 2 ) return q;
		if ( t < 2 / 3 ) return p + ( q - p ) * 6 * ( 2 / 3 - t );
		return p;

	}

	/**
	 * A Color instance is represented by RGB components in the linear <i>working
	 * color space</i>, which defaults to `LinearSRGBColorSpace`. Inputs
	 * conventionally using `SRGBColorSpace` (such as hexadecimals and CSS
	 * strings) are converted to the working color space automatically.
	 *
	 * ```js
	 * // converted automatically from SRGBColorSpace to LinearSRGBColorSpace
	 * const color = new THREE.Color().setHex( 0x112233 );
	 * ```
	 * Source color spaces may be specified explicitly, to ensure correct conversions.
	 * ```js
	 * // assumed already LinearSRGBColorSpace; no conversion
	 * const color = new THREE.Color().setRGB( 0.5, 0.5, 0.5 );
	 *
	 * // converted explicitly from SRGBColorSpace to LinearSRGBColorSpace
	 * const color = new THREE.Color().setRGB( 0.5, 0.5, 0.5, SRGBColorSpace );
	 * ```
	 * If THREE.ColorManagement is disabled, no conversions occur. For details,
	 * see <i>Color management</i>. Iterating through a Color instance will yield
	 * its components (r, g, b) in the corresponding order. A Color can be initialised
	 * in any of the following ways:
	 * ```js
	 * //empty constructor - will default white
	 * const color1 = new THREE.Color();
	 *
	 * //Hexadecimal color (recommended)
	 * const color2 = new THREE.Color( 0xff0000 );
	 *
	 * //RGB string
	 * const color3 = new THREE.Color("rgb(255, 0, 0)");
	 * const color4 = new THREE.Color("rgb(100%, 0%, 0%)");
	 *
	 * //X11 color name - all 140 color names are supported.
	 * //Note the lack of CamelCase in the name
	 * const color5 = new THREE.Color( 'skyblue' );
	 * //HSL string
	 * const color6 = new THREE.Color("hsl(0, 100%, 50%)");
	 *
	 * //Separate RGB values between 0 and 1
	 * const color7 = new THREE.Color( 1, 0, 0 );
	 * ```
	 */
	class Color {

		/**
		 * Constructs a new color.
		 *
		 * Note that standard method of specifying color in three.js is with a hexadecimal triplet,
		 * and that method is used throughout the rest of the documentation.
		 *
		 * @param {(number|string|Color)} [r] - The red component of the color. If `g` and `b` are
		 * not provided, it can be hexadecimal triplet, a CSS-style string or another `Color` instance.
		 * @param {number} [g] - The green component.
		 * @param {number} [b] - The blue component.
		 */
		constructor( r, g, b ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isColor = true;

			/**
			 * The red component.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.r = 1;

			/**
			 * The green component.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.g = 1;

			/**
			 * The blue component.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.b = 1;

			return this.set( r, g, b );

		}

		/**
		 * Sets the colors's components from the given values.
		 *
		 * @param {(number|string|Color)} [r] - The red component of the color. If `g` and `b` are
		 * not provided, it can be hexadecimal triplet, a CSS-style string or another `Color` instance.
		 * @param {number} [g] - The green component.
		 * @param {number} [b] - The blue component.
		 * @return {Color} A reference to this color.
		 */
		set( r, g, b ) {

			if ( g === undefined && b === undefined ) {

				// r is THREE.Color, hex or string

				const value = r;

				if ( value && value.isColor ) {

					this.copy( value );

				} else if ( typeof value === 'number' ) {

					this.setHex( value );

				} else if ( typeof value === 'string' ) {

					this.setStyle( value );

				}

			} else {

				this.setRGB( r, g, b );

			}

			return this;

		}

		/**
		 * Sets the colors's components to the given scalar value.
		 *
		 * @param {number} scalar - The scalar value.
		 * @return {Color} A reference to this color.
		 */
		setScalar( scalar ) {

			this.r = scalar;
			this.g = scalar;
			this.b = scalar;

			return this;

		}

		/**
		 * Sets this color from a hexadecimal value.
		 *
		 * @param {number} hex - The hexadecimal value.
		 * @param {string} [colorSpace=SRGBColorSpace] - The color space.
		 * @return {Color} A reference to this color.
		 */
		setHex( hex, colorSpace = SRGBColorSpace ) {

			hex = Math.floor( hex );

			this.r = ( hex >> 16 & 255 ) / 255;
			this.g = ( hex >> 8 & 255 ) / 255;
			this.b = ( hex & 255 ) / 255;

			ColorManagement.colorSpaceToWorking( this, colorSpace );

			return this;

		}

		/**
		 * Sets this color from RGB values.
		 *
		 * @param {number} r - Red channel value between `0.0` and `1.0`.
		 * @param {number} g - Green channel value between `0.0` and `1.0`.
		 * @param {number} b - Blue channel value between `0.0` and `1.0`.
		 * @param {string} [colorSpace=ColorManagement.workingColorSpace] - The color space.
		 * @return {Color} A reference to this color.
		 */
		setRGB( r, g, b, colorSpace = ColorManagement.workingColorSpace ) {

			this.r = r;
			this.g = g;
			this.b = b;

			ColorManagement.colorSpaceToWorking( this, colorSpace );

			return this;

		}

		/**
		 * Sets this color from RGB values.
		 *
		 * @param {number} h - Hue value between `0.0` and `1.0`.
		 * @param {number} s - Saturation value between `0.0` and `1.0`.
		 * @param {number} l - Lightness value between `0.0` and `1.0`.
		 * @param {string} [colorSpace=ColorManagement.workingColorSpace] - The color space.
		 * @return {Color} A reference to this color.
		 */
		setHSL( h, s, l, colorSpace = ColorManagement.workingColorSpace ) {

			// h,s,l ranges are in 0.0 - 1.0
			h = euclideanModulo( h, 1 );
			s = clamp( s, 0, 1 );
			l = clamp( l, 0, 1 );

			if ( s === 0 ) {

				this.r = this.g = this.b = l;

			} else {

				const p = l <= 0.5 ? l * ( 1 + s ) : l + s - ( l * s );
				const q = ( 2 * l ) - p;

				this.r = hue2rgb( q, p, h + 1 / 3 );
				this.g = hue2rgb( q, p, h );
				this.b = hue2rgb( q, p, h - 1 / 3 );

			}

			ColorManagement.colorSpaceToWorking( this, colorSpace );

			return this;

		}

		/**
		 * Sets this color from a CSS-style string. For example, `rgb(250, 0,0)`,
		 * `rgb(100%, 0%, 0%)`, `hsl(0, 100%, 50%)`, `#ff0000`, `#f00`, or `red` ( or
		 * any [X11 color name]{@link https://en.wikipedia.org/wiki/X11_color_names#Color_name_chart} -
		 * all 140 color names are supported).
		 *
		 * @param {string} style - Color as a CSS-style string.
		 * @param {string} [colorSpace=SRGBColorSpace] - The color space.
		 * @return {Color} A reference to this color.
		 */
		setStyle( style, colorSpace = SRGBColorSpace ) {

			function handleAlpha( string ) {

				if ( string === undefined ) return;

				if ( parseFloat( string ) < 1 ) {

					console.warn( 'THREE.Color: Alpha component of ' + style + ' will be ignored.' );

				}

			}


			let m;

			if ( m = /^(\w+)\(([^\)]*)\)/.exec( style ) ) {

				// rgb / hsl

				let color;
				const name = m[ 1 ];
				const components = m[ 2 ];

				switch ( name ) {

					case 'rgb':
					case 'rgba':

						if ( color = /^\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec( components ) ) {

							// rgb(255,0,0) rgba(255,0,0,0.5)

							handleAlpha( color[ 4 ] );

							return this.setRGB(
								Math.min( 255, parseInt( color[ 1 ], 10 ) ) / 255,
								Math.min( 255, parseInt( color[ 2 ], 10 ) ) / 255,
								Math.min( 255, parseInt( color[ 3 ], 10 ) ) / 255,
								colorSpace
							);

						}

						if ( color = /^\s*(\d+)\%\s*,\s*(\d+)\%\s*,\s*(\d+)\%\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec( components ) ) {

							// rgb(100%,0%,0%) rgba(100%,0%,0%,0.5)

							handleAlpha( color[ 4 ] );

							return this.setRGB(
								Math.min( 100, parseInt( color[ 1 ], 10 ) ) / 100,
								Math.min( 100, parseInt( color[ 2 ], 10 ) ) / 100,
								Math.min( 100, parseInt( color[ 3 ], 10 ) ) / 100,
								colorSpace
							);

						}

						break;

					case 'hsl':
					case 'hsla':

						if ( color = /^\s*(\d*\.?\d+)\s*,\s*(\d*\.?\d+)\%\s*,\s*(\d*\.?\d+)\%\s*(?:,\s*(\d*\.?\d+)\s*)?$/.exec( components ) ) {

							// hsl(120,50%,50%) hsla(120,50%,50%,0.5)

							handleAlpha( color[ 4 ] );

							return this.setHSL(
								parseFloat( color[ 1 ] ) / 360,
								parseFloat( color[ 2 ] ) / 100,
								parseFloat( color[ 3 ] ) / 100,
								colorSpace
							);

						}

						break;

					default:

						console.warn( 'THREE.Color: Unknown color model ' + style );

				}

			} else if ( m = /^\#([A-Fa-f\d]+)$/.exec( style ) ) {

				// hex color

				const hex = m[ 1 ];
				const size = hex.length;

				if ( size === 3 ) {

					// #ff0
					return this.setRGB(
						parseInt( hex.charAt( 0 ), 16 ) / 15,
						parseInt( hex.charAt( 1 ), 16 ) / 15,
						parseInt( hex.charAt( 2 ), 16 ) / 15,
						colorSpace
					);

				} else if ( size === 6 ) {

					// #ff0000
					return this.setHex( parseInt( hex, 16 ), colorSpace );

				} else {

					console.warn( 'THREE.Color: Invalid hex color ' + style );

				}

			} else if ( style && style.length > 0 ) {

				return this.setColorName( style, colorSpace );

			}

			return this;

		}

		/**
		 * Sets this color from a color name. Faster than {@link Color#setStyle} if
		 * you don't need the other CSS-style formats.
		 *
		 * For convenience, the list of names is exposed in `Color.NAMES` as a hash.
		 * ```js
		 * Color.NAMES.aliceblue // returns 0xF0F8FF
		 * ```
		 *
		 * @param {string} style - The color name.
		 * @param {string} [colorSpace=SRGBColorSpace] - The color space.
		 * @return {Color} A reference to this color.
		 */
		setColorName( style, colorSpace = SRGBColorSpace ) {

			// color keywords
			const hex = _colorKeywords[ style.toLowerCase() ];

			if ( hex !== undefined ) {

				// red
				this.setHex( hex, colorSpace );

			} else {

				// unknown color
				console.warn( 'THREE.Color: Unknown color ' + style );

			}

			return this;

		}

		/**
		 * Returns a new color with copied values from this instance.
		 *
		 * @return {Color} A clone of this instance.
		 */
		clone() {

			return new this.constructor( this.r, this.g, this.b );

		}

		/**
		 * Copies the values of the given color to this instance.
		 *
		 * @param {Color} color - The color to copy.
		 * @return {Color} A reference to this color.
		 */
		copy( color ) {

			this.r = color.r;
			this.g = color.g;
			this.b = color.b;

			return this;

		}

		/**
		 * Copies the given color into this color, and then converts this color from
		 * `SRGBColorSpace` to `LinearSRGBColorSpace`.
		 *
		 * @param {Color} color - The color to copy/convert.
		 * @return {Color} A reference to this color.
		 */
		copySRGBToLinear( color ) {

			this.r = SRGBToLinear( color.r );
			this.g = SRGBToLinear( color.g );
			this.b = SRGBToLinear( color.b );

			return this;

		}

		/**
		 * Copies the given color into this color, and then converts this color from
		 * `LinearSRGBColorSpace` to `SRGBColorSpace`.
		 *
		 * @param {Color} color - The color to copy/convert.
		 * @return {Color} A reference to this color.
		 */
		copyLinearToSRGB( color ) {

			this.r = LinearToSRGB( color.r );
			this.g = LinearToSRGB( color.g );
			this.b = LinearToSRGB( color.b );

			return this;

		}

		/**
		 * Converts this color from `SRGBColorSpace` to `LinearSRGBColorSpace`.
		 *
		 * @return {Color} A reference to this color.
		 */
		convertSRGBToLinear() {

			this.copySRGBToLinear( this );

			return this;

		}

		/**
		 * Converts this color from `LinearSRGBColorSpace` to `SRGBColorSpace`.
		 *
		 * @return {Color} A reference to this color.
		 */
		convertLinearToSRGB() {

			this.copyLinearToSRGB( this );

			return this;

		}

		/**
		 * Returns the hexadecimal value of this color.
		 *
		 * @param {string} [colorSpace=SRGBColorSpace] - The color space.
		 * @return {number} The hexadecimal value.
		 */
		getHex( colorSpace = SRGBColorSpace ) {

			ColorManagement.workingToColorSpace( _color.copy( this ), colorSpace );

			return Math.round( clamp( _color.r * 255, 0, 255 ) ) * 65536 + Math.round( clamp( _color.g * 255, 0, 255 ) ) * 256 + Math.round( clamp( _color.b * 255, 0, 255 ) );

		}

		/**
		 * Returns the hexadecimal value of this color as a string (for example, 'FFFFFF').
		 *
		 * @param {string} [colorSpace=SRGBColorSpace] - The color space.
		 * @return {string} The hexadecimal value as a string.
		 */
		getHexString( colorSpace = SRGBColorSpace ) {

			return ( '000000' + this.getHex( colorSpace ).toString( 16 ) ).slice( - 6 );

		}

		/**
		 * Converts the colors RGB values into the HSL format and stores them into the
		 * given target object.
		 *
		 * @param {{h:number,s:number,l:number}} target - The target object that is used to store the method's result.
		 * @param {string} [colorSpace=ColorManagement.workingColorSpace] - The color space.
		 * @return {{h:number,s:number,l:number}} The HSL representation of this color.
		 */
		getHSL( target, colorSpace = ColorManagement.workingColorSpace ) {

			// h,s,l ranges are in 0.0 - 1.0

			ColorManagement.workingToColorSpace( _color.copy( this ), colorSpace );

			const r = _color.r, g = _color.g, b = _color.b;

			const max = Math.max( r, g, b );
			const min = Math.min( r, g, b );

			let hue, saturation;
			const lightness = ( min + max ) / 2.0;

			if ( min === max ) {

				hue = 0;
				saturation = 0;

			} else {

				const delta = max - min;

				saturation = lightness <= 0.5 ? delta / ( max + min ) : delta / ( 2 - max - min );

				switch ( max ) {

					case r: hue = ( g - b ) / delta + ( g < b ? 6 : 0 ); break;
					case g: hue = ( b - r ) / delta + 2; break;
					case b: hue = ( r - g ) / delta + 4; break;

				}

				hue /= 6;

			}

			target.h = hue;
			target.s = saturation;
			target.l = lightness;

			return target;

		}

		/**
		 * Returns the RGB values of this color and stores them into the given target object.
		 *
		 * @param {Color} target - The target color that is used to store the method's result.
		 * @param {string} [colorSpace=ColorManagement.workingColorSpace] - The color space.
		 * @return {Color} The RGB representation of this color.
		 */
		getRGB( target, colorSpace = ColorManagement.workingColorSpace ) {

			ColorManagement.workingToColorSpace( _color.copy( this ), colorSpace );

			target.r = _color.r;
			target.g = _color.g;
			target.b = _color.b;

			return target;

		}

		/**
		 * Returns the value of this color as a CSS style string. Example: `rgb(255,0,0)`.
		 *
		 * @param {string} [colorSpace=SRGBColorSpace] - The color space.
		 * @return {string} The CSS representation of this color.
		 */
		getStyle( colorSpace = SRGBColorSpace ) {

			ColorManagement.workingToColorSpace( _color.copy( this ), colorSpace );

			const r = _color.r, g = _color.g, b = _color.b;

			if ( colorSpace !== SRGBColorSpace ) {

				// Requires CSS Color Module Level 4 (https://www.w3.org/TR/css-color-4/).
				return `color(${ colorSpace } ${ r.toFixed( 3 ) } ${ g.toFixed( 3 ) } ${ b.toFixed( 3 ) })`;

			}

			return `rgb(${ Math.round( r * 255 ) },${ Math.round( g * 255 ) },${ Math.round( b * 255 ) })`;

		}

		/**
		 * Adds the given HSL values to this color's values.
		 * Internally, this converts the color's RGB values to HSL, adds HSL
		 * and then converts the color back to RGB.
		 *
		 * @param {number} h - Hue value between `0.0` and `1.0`.
		 * @param {number} s - Saturation value between `0.0` and `1.0`.
		 * @param {number} l - Lightness value between `0.0` and `1.0`.
		 * @return {Color} A reference to this color.
		 */
		offsetHSL( h, s, l ) {

			this.getHSL( _hslA );

			return this.setHSL( _hslA.h + h, _hslA.s + s, _hslA.l + l );

		}

		/**
		 * Adds the RGB values of the given color to the RGB values of this color.
		 *
		 * @param {Color} color - The color to add.
		 * @return {Color} A reference to this color.
		 */
		add( color ) {

			this.r += color.r;
			this.g += color.g;
			this.b += color.b;

			return this;

		}

		/**
		 * Adds the RGB values of the given colors and stores the result in this instance.
		 *
		 * @param {Color} color1 - The first color.
		 * @param {Color} color2 - The second color.
		 * @return {Color} A reference to this color.
		 */
		addColors( color1, color2 ) {

			this.r = color1.r + color2.r;
			this.g = color1.g + color2.g;
			this.b = color1.b + color2.b;

			return this;

		}

		/**
		 * Adds the given scalar value to the RGB values of this color.
		 *
		 * @param {number} s - The scalar to add.
		 * @return {Color} A reference to this color.
		 */
		addScalar( s ) {

			this.r += s;
			this.g += s;
			this.b += s;

			return this;

		}

		/**
		 * Subtracts the RGB values of the given color from the RGB values of this color.
		 *
		 * @param {Color} color - The color to subtract.
		 * @return {Color} A reference to this color.
		 */
		sub( color ) {

			this.r = Math.max( 0, this.r - color.r );
			this.g = Math.max( 0, this.g - color.g );
			this.b = Math.max( 0, this.b - color.b );

			return this;

		}

		/**
		 * Multiplies the RGB values of the given color with the RGB values of this color.
		 *
		 * @param {Color} color - The color to multiply.
		 * @return {Color} A reference to this color.
		 */
		multiply( color ) {

			this.r *= color.r;
			this.g *= color.g;
			this.b *= color.b;

			return this;

		}

		/**
		 * Multiplies the given scalar value with the RGB values of this color.
		 *
		 * @param {number} s - The scalar to multiply.
		 * @return {Color} A reference to this color.
		 */
		multiplyScalar( s ) {

			this.r *= s;
			this.g *= s;
			this.b *= s;

			return this;

		}

		/**
		 * Linearly interpolates this color's RGB values toward the RGB values of the
		 * given color. The alpha argument can be thought of as the ratio between
		 * the two colors, where `0.0` is this color and `1.0` is the first argument.
		 *
		 * @param {Color} color - The color to converge on.
		 * @param {number} alpha - The interpolation factor in the closed interval `[0,1]`.
		 * @return {Color} A reference to this color.
		 */
		lerp( color, alpha ) {

			this.r += ( color.r - this.r ) * alpha;
			this.g += ( color.g - this.g ) * alpha;
			this.b += ( color.b - this.b ) * alpha;

			return this;

		}

		/**
		 * Linearly interpolates between the given colors and stores the result in this instance.
		 * The alpha argument can be thought of as the ratio between the two colors, where `0.0`
		 * is the first and `1.0` is the second color.
		 *
		 * @param {Color} color1 - The first color.
		 * @param {Color} color2 - The second color.
		 * @param {number} alpha - The interpolation factor in the closed interval `[0,1]`.
		 * @return {Color} A reference to this color.
		 */
		lerpColors( color1, color2, alpha ) {

			this.r = color1.r + ( color2.r - color1.r ) * alpha;
			this.g = color1.g + ( color2.g - color1.g ) * alpha;
			this.b = color1.b + ( color2.b - color1.b ) * alpha;

			return this;

		}

		/**
		 * Linearly interpolates this color's HSL values toward the HSL values of the
		 * given color. It differs from {@link Color#lerp} by not interpolating straight
		 * from one color to the other, but instead going through all the hues in between
		 * those two colors. The alpha argument can be thought of as the ratio between
		 * the two colors, where 0.0 is this color and 1.0 is the first argument.
		 *
		 * @param {Color} color - The color to converge on.
		 * @param {number} alpha - The interpolation factor in the closed interval `[0,1]`.
		 * @return {Color} A reference to this color.
		 */
		lerpHSL( color, alpha ) {

			this.getHSL( _hslA );
			color.getHSL( _hslB );

			const h = lerp( _hslA.h, _hslB.h, alpha );
			const s = lerp( _hslA.s, _hslB.s, alpha );
			const l = lerp( _hslA.l, _hslB.l, alpha );

			this.setHSL( h, s, l );

			return this;

		}

		/**
		 * Sets the color's RGB components from the given 3D vector.
		 *
		 * @param {Vector3} v - The vector to set.
		 * @return {Color} A reference to this color.
		 */
		setFromVector3( v ) {

			this.r = v.x;
			this.g = v.y;
			this.b = v.z;

			return this;

		}

		/**
		 * Transforms this color with the given 3x3 matrix.
		 *
		 * @param {Matrix3} m - The matrix.
		 * @return {Color} A reference to this color.
		 */
		applyMatrix3( m ) {

			const r = this.r, g = this.g, b = this.b;
			const e = m.elements;

			this.r = e[ 0 ] * r + e[ 3 ] * g + e[ 6 ] * b;
			this.g = e[ 1 ] * r + e[ 4 ] * g + e[ 7 ] * b;
			this.b = e[ 2 ] * r + e[ 5 ] * g + e[ 8 ] * b;

			return this;

		}

		/**
		 * Returns `true` if this color is equal with the given one.
		 *
		 * @param {Color} c - The color to test for equality.
		 * @return {boolean} Whether this bounding color is equal with the given one.
		 */
		equals( c ) {

			return ( c.r === this.r ) && ( c.g === this.g ) && ( c.b === this.b );

		}

		/**
		 * Sets this color's RGB components from the given array.
		 *
		 * @param {Array<number>} array - An array holding the RGB values.
		 * @param {number} [offset=0] - The offset into the array.
		 * @return {Color} A reference to this color.
		 */
		fromArray( array, offset = 0 ) {

			this.r = array[ offset ];
			this.g = array[ offset + 1 ];
			this.b = array[ offset + 2 ];

			return this;

		}

		/**
		 * Writes the RGB components of this color to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number>} [array=[]] - The target array holding the color components.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number>} The color components.
		 */
		toArray( array = [], offset = 0 ) {

			array[ offset ] = this.r;
			array[ offset + 1 ] = this.g;
			array[ offset + 2 ] = this.b;

			return array;

		}

		/**
		 * Sets the components of this color from the given buffer attribute.
		 *
		 * @param {BufferAttribute} attribute - The buffer attribute holding color data.
		 * @param {number} index - The index into the attribute.
		 * @return {Color} A reference to this color.
		 */
		fromBufferAttribute( attribute, index ) {

			this.r = attribute.getX( index );
			this.g = attribute.getY( index );
			this.b = attribute.getZ( index );

			return this;

		}

		/**
		 * This methods defines the serialization result of this class. Returns the color
		 * as a hexadecimal value.
		 *
		 * @return {number} The hexadecimal value.
		 */
		toJSON() {

			return this.getHex();

		}

		*[ Symbol.iterator ]() {

			yield this.r;
			yield this.g;
			yield this.b;

		}

	}

	const _color = /*@__PURE__*/ new Color();

	/**
	 * A dictionary with X11 color names.
	 *
	 * Note that multiple words such as Dark Orange become the string 'darkorange'.
	 *
	 * @static
	 * @type {Object}
	 */
	Color.NAMES = _colorKeywords;

	/**
	 * This modules allows to dispatch event objects on custom JavaScript objects.
	 *
	 * Main repository: [eventdispatcher.js]{@link https://github.com/mrdoob/eventdispatcher.js/}
	 *
	 * Code Example:
	 * ```js
	 * class Car extends EventDispatcher {
	 * 	start() {
	 *		this.dispatchEvent( { type: 'start', message: 'vroom vroom!' } );
	 *	}
	 *};
	 *
	 * // Using events with the custom object
	 * const car = new Car();
	 * car.addEventListener( 'start', function ( event ) {
	 * 	alert( event.message );
	 * } );
	 *
	 * car.start();
	 * ```
	 */
	class EventDispatcher {

		/**
		 * Adds the given event listener to the given event type.
		 *
		 * @param {string} type - The type of event to listen to.
		 * @param {Function} listener - The function that gets called when the event is fired.
		 */
		addEventListener( type, listener ) {

			if ( this._listeners === undefined ) this._listeners = {};

			const listeners = this._listeners;

			if ( listeners[ type ] === undefined ) {

				listeners[ type ] = [];

			}

			if ( listeners[ type ].indexOf( listener ) === - 1 ) {

				listeners[ type ].push( listener );

			}

		}

		/**
		 * Returns `true` if the given event listener has been added to the given event type.
		 *
		 * @param {string} type - The type of event.
		 * @param {Function} listener - The listener to check.
		 * @return {boolean} Whether the given event listener has been added to the given event type.
		 */
		hasEventListener( type, listener ) {

			const listeners = this._listeners;

			if ( listeners === undefined ) return false;

			return listeners[ type ] !== undefined && listeners[ type ].indexOf( listener ) !== - 1;

		}

		/**
		 * Removes the given event listener from the given event type.
		 *
		 * @param {string} type - The type of event.
		 * @param {Function} listener - The listener to remove.
		 */
		removeEventListener( type, listener ) {

			const listeners = this._listeners;

			if ( listeners === undefined ) return;

			const listenerArray = listeners[ type ];

			if ( listenerArray !== undefined ) {

				const index = listenerArray.indexOf( listener );

				if ( index !== - 1 ) {

					listenerArray.splice( index, 1 );

				}

			}

		}

		/**
		 * Dispatches an event object.
		 *
		 * @param {Object} event - The event that gets fired.
		 */
		dispatchEvent( event ) {

			const listeners = this._listeners;

			if ( listeners === undefined ) return;

			const listenerArray = listeners[ event.type ];

			if ( listenerArray !== undefined ) {

				event.target = this;

				// Make a copy, in case listeners are removed while iterating.
				const array = listenerArray.slice( 0 );

				for ( let i = 0, l = array.length; i < l; i ++ ) {

					array[ i ].call( this, event );

				}

				event.target = null;

			}

		}

	}

	const _matrix$1 = /*@__PURE__*/ new Matrix4();
	const _quaternion$1 = /*@__PURE__*/ new Quaternion();

	/**
	 * A class representing Euler angles.
	 *
	 * Euler angles describe a rotational transformation by rotating an object on
	 * its various axes in specified amounts per axis, and a specified axis
	 * order.
	 *
	 * Iterating through an instance will yield its components (x, y, z,
	 * order) in the corresponding order.
	 *
	 * ```js
	 * const a = new THREE.Euler( 0, 1, 1.57, 'XYZ' );
	 * const b = new THREE.Vector3( 1, 0, 1 );
	 * b.applyEuler(a);
	 * ```
	 */
	class Euler {

		/**
		 * Constructs a new euler instance.
		 *
		 * @param {number} [x=0] - The angle of the x axis in radians.
		 * @param {number} [y=0] - The angle of the y axis in radians.
		 * @param {number} [z=0] - The angle of the z axis in radians.
		 * @param {string} [order=Euler.DEFAULT_ORDER] - A string representing the order that the rotations are applied.
		 */
		constructor( x = 0, y = 0, z = 0, order = Euler.DEFAULT_ORDER ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isEuler = true;

			this._x = x;
			this._y = y;
			this._z = z;
			this._order = order;

		}

		/**
		 * The angle of the x axis in radians.
		 *
		 * @type {number}
		 * @default 0
		 */
		get x() {

			return this._x;

		}

		set x( value ) {

			this._x = value;
			this._onChangeCallback();

		}

		/**
		 * The angle of the y axis in radians.
		 *
		 * @type {number}
		 * @default 0
		 */
		get y() {

			return this._y;

		}

		set y( value ) {

			this._y = value;
			this._onChangeCallback();

		}

		/**
		 * The angle of the z axis in radians.
		 *
		 * @type {number}
		 * @default 0
		 */
		get z() {

			return this._z;

		}

		set z( value ) {

			this._z = value;
			this._onChangeCallback();

		}

		/**
		 * A string representing the order that the rotations are applied.
		 *
		 * @type {string}
		 * @default 'XYZ'
		 */
		get order() {

			return this._order;

		}

		set order( value ) {

			this._order = value;
			this._onChangeCallback();

		}

		/**
		 * Sets the Euler components.
		 *
		 * @param {number} x - The angle of the x axis in radians.
		 * @param {number} y - The angle of the y axis in radians.
		 * @param {number} z - The angle of the z axis in radians.
		 * @param {string} [order] - A string representing the order that the rotations are applied.
		 * @return {Euler} A reference to this Euler instance.
		 */
		set( x, y, z, order = this._order ) {

			this._x = x;
			this._y = y;
			this._z = z;
			this._order = order;

			this._onChangeCallback();

			return this;

		}

		/**
		 * Returns a new Euler instance with copied values from this instance.
		 *
		 * @return {Euler} A clone of this instance.
		 */
		clone() {

			return new this.constructor( this._x, this._y, this._z, this._order );

		}

		/**
		 * Copies the values of the given Euler instance to this instance.
		 *
		 * @param {Euler} euler - The Euler instance to copy.
		 * @return {Euler} A reference to this Euler instance.
		 */
		copy( euler ) {

			this._x = euler._x;
			this._y = euler._y;
			this._z = euler._z;
			this._order = euler._order;

			this._onChangeCallback();

			return this;

		}

		/**
		 * Sets the angles of this Euler instance from a pure rotation matrix.
		 *
		 * @param {Matrix4} m - A 4x4 matrix of which the upper 3x3 of matrix is a pure rotation matrix (i.e. unscaled).
		 * @param {string} [order] - A string representing the order that the rotations are applied.
		 * @param {boolean} [update=true] - Whether the internal `onChange` callback should be executed or not.
		 * @return {Euler} A reference to this Euler instance.
		 */
		setFromRotationMatrix( m, order = this._order, update = true ) {

			const te = m.elements;
			const m11 = te[ 0 ], m12 = te[ 4 ], m13 = te[ 8 ];
			const m21 = te[ 1 ], m22 = te[ 5 ], m23 = te[ 9 ];
			const m31 = te[ 2 ], m32 = te[ 6 ], m33 = te[ 10 ];

			switch ( order ) {

				case 'XYZ':

					this._y = Math.asin( clamp( m13, - 1, 1 ) );

					if ( Math.abs( m13 ) < 0.9999999 ) {

						this._x = Math.atan2( - m23, m33 );
						this._z = Math.atan2( - m12, m11 );

					} else {

						this._x = Math.atan2( m32, m22 );
						this._z = 0;

					}

					break;

				case 'YXZ':

					this._x = Math.asin( - clamp( m23, - 1, 1 ) );

					if ( Math.abs( m23 ) < 0.9999999 ) {

						this._y = Math.atan2( m13, m33 );
						this._z = Math.atan2( m21, m22 );

					} else {

						this._y = Math.atan2( - m31, m11 );
						this._z = 0;

					}

					break;

				case 'ZXY':

					this._x = Math.asin( clamp( m32, - 1, 1 ) );

					if ( Math.abs( m32 ) < 0.9999999 ) {

						this._y = Math.atan2( - m31, m33 );
						this._z = Math.atan2( - m12, m22 );

					} else {

						this._y = 0;
						this._z = Math.atan2( m21, m11 );

					}

					break;

				case 'ZYX':

					this._y = Math.asin( - clamp( m31, - 1, 1 ) );

					if ( Math.abs( m31 ) < 0.9999999 ) {

						this._x = Math.atan2( m32, m33 );
						this._z = Math.atan2( m21, m11 );

					} else {

						this._x = 0;
						this._z = Math.atan2( - m12, m22 );

					}

					break;

				case 'YZX':

					this._z = Math.asin( clamp( m21, - 1, 1 ) );

					if ( Math.abs( m21 ) < 0.9999999 ) {

						this._x = Math.atan2( - m23, m22 );
						this._y = Math.atan2( - m31, m11 );

					} else {

						this._x = 0;
						this._y = Math.atan2( m13, m33 );

					}

					break;

				case 'XZY':

					this._z = Math.asin( - clamp( m12, - 1, 1 ) );

					if ( Math.abs( m12 ) < 0.9999999 ) {

						this._x = Math.atan2( m32, m22 );
						this._y = Math.atan2( m13, m11 );

					} else {

						this._x = Math.atan2( - m23, m33 );
						this._y = 0;

					}

					break;

				default:

					console.warn( 'THREE.Euler: .setFromRotationMatrix() encountered an unknown order: ' + order );

			}

			this._order = order;

			if ( update === true ) this._onChangeCallback();

			return this;

		}

		/**
		 * Sets the angles of this Euler instance from a normalized quaternion.
		 *
		 * @param {Quaternion} q - A normalized Quaternion.
		 * @param {string} [order] - A string representing the order that the rotations are applied.
		 * @param {boolean} [update=true] - Whether the internal `onChange` callback should be executed or not.
		 * @return {Euler} A reference to this Euler instance.
		 */
		setFromQuaternion( q, order, update ) {

			_matrix$1.makeRotationFromQuaternion( q );

			return this.setFromRotationMatrix( _matrix$1, order, update );

		}

		/**
		 * Sets the angles of this Euler instance from the given vector.
		 *
		 * @param {Vector3} v - The vector.
		 * @param {string} [order] - A string representing the order that the rotations are applied.
		 * @return {Euler} A reference to this Euler instance.
		 */
		setFromVector3( v, order = this._order ) {

			return this.set( v.x, v.y, v.z, order );

		}

		/**
		 * Resets the euler angle with a new order by creating a quaternion from this
		 * euler angle and then setting this euler angle with the quaternion and the
		 * new order.
		 *
		 * Warning: This discards revolution information.
		 *
		 * @param {string} [newOrder] - A string representing the new order that the rotations are applied.
		 * @return {Euler} A reference to this Euler instance.
		 */
		reorder( newOrder ) {

			_quaternion$1.setFromEuler( this );

			return this.setFromQuaternion( _quaternion$1, newOrder );

		}

		/**
		 * Returns `true` if this Euler instance is equal with the given one.
		 *
		 * @param {Euler} euler - The Euler instance to test for equality.
		 * @return {boolean} Whether this Euler instance is equal with the given one.
		 */
		equals( euler ) {

			return ( euler._x === this._x ) && ( euler._y === this._y ) && ( euler._z === this._z ) && ( euler._order === this._order );

		}

		/**
		 * Sets this Euler instance's components to values from the given array. The first three
		 * entries of the array are assign to the x,y and z components. An optional fourth entry
		 * defines the Euler order.
		 *
		 * @param {Array<number,number,number,?string>} array - An array holding the Euler component values.
		 * @return {Euler} A reference to this Euler instance.
		 */
		fromArray( array ) {

			this._x = array[ 0 ];
			this._y = array[ 1 ];
			this._z = array[ 2 ];
			if ( array[ 3 ] !== undefined ) this._order = array[ 3 ];

			this._onChangeCallback();

			return this;

		}

		/**
		 * Writes the components of this Euler instance to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number,number,number,string>} [array=[]] - The target array holding the Euler components.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number,number,number,string>} The Euler components.
		 */
		toArray( array = [], offset = 0 ) {

			array[ offset ] = this._x;
			array[ offset + 1 ] = this._y;
			array[ offset + 2 ] = this._z;
			array[ offset + 3 ] = this._order;

			return array;

		}

		_onChange( callback ) {

			this._onChangeCallback = callback;

			return this;

		}

		_onChangeCallback() {}

		*[ Symbol.iterator ]() {

			yield this._x;
			yield this._y;
			yield this._z;
			yield this._order;

		}

	}

	/**
	 * The default Euler angle order.
	 *
	 * @static
	 * @type {string}
	 * @default 'XYZ'
	 */
	Euler.DEFAULT_ORDER = 'XYZ';

	/**
	 * A layers object assigns an 3D object to 1 or more of 32
	 * layers numbered `0` to `31` - internally the layers are stored as a
	 * bit mask], and by default all 3D objects are a member of layer `0`.
	 *
	 * This can be used to control visibility - an object must share a layer with
	 * a camera to be visible when that camera's view is
	 * rendered.
	 *
	 * All classes that inherit from {@link Object3D} have an `layers` property which
	 * is an instance of this class.
	 */
	class Layers {

		/**
		 * Constructs a new layers instance, with membership
		 * initially set to layer `0`.
		 */
		constructor() {

			/**
			 * A bit mask storing which of the 32 layers this layers object is currently
			 * a member of.
			 *
			 * @type {number}
			 */
			this.mask = 1 | 0;

		}

		/**
		 * Sets membership to the given layer, and remove membership all other layers.
		 *
		 * @param {number} layer - The layer to set.
		 */
		set( layer ) {

			this.mask = ( 1 << layer | 0 ) >>> 0;

		}

		/**
		 * Adds membership of the given layer.
		 *
		 * @param {number} layer - The layer to enable.
		 */
		enable( layer ) {

			this.mask |= 1 << layer | 0;

		}

		/**
		 * Adds membership to all layers.
		 */
		enableAll() {

			this.mask = 0xffffffff | 0;

		}

		/**
		 * Toggles the membership of the given layer.
		 *
		 * @param {number} layer - The layer to toggle.
		 */
		toggle( layer ) {

			this.mask ^= 1 << layer | 0;

		}

		/**
		 * Removes membership of the given layer.
		 *
		 * @param {number} layer - The layer to enable.
		 */
		disable( layer ) {

			this.mask &= ~ ( 1 << layer | 0 );

		}

		/**
		 * Removes the membership from all layers.
		 */
		disableAll() {

			this.mask = 0;

		}

		/**
		 * Returns `true` if this and the given layers object have at least one
		 * layer in common.
		 *
		 * @param {Layers} layers - The layers to test.
		 * @return {boolean } Whether this and the given layers object have at least one layer in common or not.
		 */
		test( layers ) {

			return ( this.mask & layers.mask ) !== 0;

		}

		/**
		 * Returns `true` if the given layer is enabled.
		 *
		 * @param {number} layer - The layer to test.
		 * @return {boolean } Whether the given layer is enabled or not.
		 */
		isEnabled( layer ) {

			return ( this.mask & ( 1 << layer | 0 ) ) !== 0;

		}

	}

	let _object3DId = 0;

	const _v1$2 = /*@__PURE__*/ new Vector3();
	const _q1 = /*@__PURE__*/ new Quaternion();
	const _m1$1 = /*@__PURE__*/ new Matrix4();
	const _target = /*@__PURE__*/ new Vector3();

	const _position = /*@__PURE__*/ new Vector3();
	const _scale = /*@__PURE__*/ new Vector3();
	const _quaternion = /*@__PURE__*/ new Quaternion();

	const _xAxis = /*@__PURE__*/ new Vector3( 1, 0, 0 );
	const _yAxis = /*@__PURE__*/ new Vector3( 0, 1, 0 );
	const _zAxis = /*@__PURE__*/ new Vector3( 0, 0, 1 );

	/**
	 * Fires when the object has been added to its parent object.
	 *
	 * @event Object3D#added
	 * @type {Object}
	 */
	const _addedEvent = { type: 'added' };

	/**
	 * Fires when the object has been removed from its parent object.
	 *
	 * @event Object3D#removed
	 * @type {Object}
	 */
	const _removedEvent = { type: 'removed' };

	/**
	 * Fires when a new child object has been added.
	 *
	 * @event Object3D#childadded
	 * @type {Object}
	 */
	const _childaddedEvent = { type: 'childadded', child: null };

	/**
	 * Fires when a child object has been removed.
	 *
	 * @event Object3D#childremoved
	 * @type {Object}
	 */
	const _childremovedEvent = { type: 'childremoved', child: null };

	/**
	 * This is the base class for most objects in three.js and provides a set of
	 * properties and methods for manipulating objects in 3D space.
	 *
	 * @augments EventDispatcher
	 */
	class Object3D extends EventDispatcher {

		/**
		 * Constructs a new 3D object.
		 */
		constructor() {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isObject3D = true;

			/**
			 * The ID of the 3D object.
			 *
			 * @name Object3D#id
			 * @type {number}
			 * @readonly
			 */
			Object.defineProperty( this, 'id', { value: _object3DId ++ } );

			/**
			 * The UUID of the 3D object.
			 *
			 * @type {string}
			 * @readonly
			 */
			this.uuid = generateUUID();

			/**
			 * The name of the 3D object.
			 *
			 * @type {string}
			 */
			this.name = '';

			/**
			 * The type property is used for detecting the object type
			 * in context of serialization/deserialization.
			 *
			 * @type {string}
			 * @readonly
			 */
			this.type = 'Object3D';

			/**
			 * A reference to the parent object.
			 *
			 * @type {?Object3D}
			 * @default null
			 */
			this.parent = null;

			/**
			 * An array holding the child 3D objects of this instance.
			 *
			 * @type {Array<Object3D>}
			 */
			this.children = [];

			/**
			 * Defines the `up` direction of the 3D object which influences
			 * the orientation via methods like {@link Object3D#lookAt}.
			 *
			 * The default values for all 3D objects is defined by `Object3D.DEFAULT_UP`.
			 *
			 * @type {Vector3}
			 */
			this.up = Object3D.DEFAULT_UP.clone();

			const position = new Vector3();
			const rotation = new Euler();
			const quaternion = new Quaternion();
			const scale = new Vector3( 1, 1, 1 );

			function onRotationChange() {

				quaternion.setFromEuler( rotation, false );

			}

			function onQuaternionChange() {

				rotation.setFromQuaternion( quaternion, undefined, false );

			}

			rotation._onChange( onRotationChange );
			quaternion._onChange( onQuaternionChange );

			Object.defineProperties( this, {
				/**
				 * Represents the object's local position.
				 *
				 * @name Object3D#position
				 * @type {Vector3}
				 * @default (0,0,0)
				 */
				position: {
					configurable: true,
					enumerable: true,
					value: position
				},
				/**
				 * Represents the object's local rotation as Euler angles, in radians.
				 *
				 * @name Object3D#rotation
				 * @type {Euler}
				 * @default (0,0,0)
				 */
				rotation: {
					configurable: true,
					enumerable: true,
					value: rotation
				},
				/**
				 * Represents the object's local rotation as Quaternions.
				 *
				 * @name Object3D#quaternion
				 * @type {Quaternion}
				 */
				quaternion: {
					configurable: true,
					enumerable: true,
					value: quaternion
				},
				/**
				 * Represents the object's local scale.
				 *
				 * @name Object3D#scale
				 * @type {Vector3}
				 * @default (1,1,1)
				 */
				scale: {
					configurable: true,
					enumerable: true,
					value: scale
				},
				/**
				 * Represents the object's model-view matrix.
				 *
				 * @name Object3D#modelViewMatrix
				 * @type {Matrix4}
				 */
				modelViewMatrix: {
					value: new Matrix4()
				},
				/**
				 * Represents the object's normal matrix.
				 *
				 * @name Object3D#normalMatrix
				 * @type {Matrix3}
				 */
				normalMatrix: {
					value: new Matrix3()
				}
			} );

			/**
			 * Represents the object's transformation matrix in local space.
			 *
			 * @type {Matrix4}
			 */
			this.matrix = new Matrix4();

			/**
			 * Represents the object's transformation matrix in world space.
			 * If the 3D object has no parent, then it's identical to the local transformation matrix
			 *
			 * @type {Matrix4}
			 */
			this.matrixWorld = new Matrix4();

			/**
			 * When set to `true`, the engine automatically computes the local matrix from position,
			 * rotation and scale every frame.
			 *
			 * The default values for all 3D objects is defined by `Object3D.DEFAULT_MATRIX_AUTO_UPDATE`.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.matrixAutoUpdate = Object3D.DEFAULT_MATRIX_AUTO_UPDATE;

			/**
			 * When set to `true`, the engine automatically computes the world matrix from the current local
			 * matrix and the object's transformation hierarchy.
			 *
			 * The default values for all 3D objects is defined by `Object3D.DEFAULT_MATRIX_WORLD_AUTO_UPDATE`.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.matrixWorldAutoUpdate = Object3D.DEFAULT_MATRIX_WORLD_AUTO_UPDATE; // checked by the renderer

			/**
			 * When set to `true`, it calculates the world matrix in that frame and resets this property
			 * to `false`.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.matrixWorldNeedsUpdate = false;

			/**
			 * The layer membership of the 3D object. The 3D object is only visible if it has
			 * at least one layer in common with the camera in use. This property can also be
			 * used to filter out unwanted objects in ray-intersection tests when using {@link Raycaster}.
			 *
			 * @type {Layers}
			 */
			this.layers = new Layers();

			/**
			 * When set to `true`, the 3D object gets rendered.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.visible = true;

			/**
			 * When set to `true`, the 3D object gets rendered into shadow maps.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.castShadow = false;

			/**
			 * When set to `true`, the 3D object is affected by shadows in the scene.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.receiveShadow = false;

			/**
			 * When set to `true`, the 3D object is honored by view frustum culling.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.frustumCulled = true;

			/**
			 * This value allows the default rendering order of scene graph objects to be
			 * overridden although opaque and transparent objects remain sorted independently.
			 * When this property is set for an instance of {@link Group},all descendants
			 * objects will be sorted and rendered together. Sorting is from lowest to highest
			 * render order.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.renderOrder = 0;

			/**
			 * An array holding the animation clips of the 3D object.
			 *
			 * @type {Array<AnimationClip>}
			 */
			this.animations = [];

			/**
			 * Custom depth material to be used when rendering to the depth map. Can only be used
			 * in context of meshes. When shadow-casting with a {@link DirectionalLight} or {@link SpotLight},
			 * if you are modifying vertex positions in the vertex shader you must specify a custom depth
			 * material for proper shadows.
			 *
			 * Only relevant in context of {@link WebGLRenderer}.
			 *
			 * @type {(Material|undefined)}
			 * @default undefined
			 */
			this.customDepthMaterial = undefined;

			/**
			 * Same as {@link Object3D#customDepthMaterial}, but used with {@link PointLight}.
			 *
			 * Only relevant in context of {@link WebGLRenderer}.
			 *
			 * @type {(Material|undefined)}
			 * @default undefined
			 */
			this.customDistanceMaterial = undefined;

			/**
			 * An object that can be used to store custom data about the 3D object. It
			 * should not hold references to functions as these will not be cloned.
			 *
			 * @type {Object}
			 */
			this.userData = {};

		}

		/**
		 * A callback that is executed immediately before a 3D object is rendered to a shadow map.
		 *
		 * @param {Renderer|WebGLRenderer} renderer - The renderer.
		 * @param {Object3D} object - The 3D object.
		 * @param {Camera} camera - The camera that is used to render the scene.
		 * @param {Camera} shadowCamera - The shadow camera.
		 * @param {BufferGeometry} geometry - The 3D object's geometry.
		 * @param {Material} depthMaterial - The depth material.
		 * @param {Object} group - The geometry group data.
		 */
		onBeforeShadow( /* renderer, object, camera, shadowCamera, geometry, depthMaterial, group */ ) {}

		/**
		 * A callback that is executed immediately after a 3D object is rendered to a shadow map.
		 *
		 * @param {Renderer|WebGLRenderer} renderer - The renderer.
		 * @param {Object3D} object - The 3D object.
		 * @param {Camera} camera - The camera that is used to render the scene.
		 * @param {Camera} shadowCamera - The shadow camera.
		 * @param {BufferGeometry} geometry - The 3D object's geometry.
		 * @param {Material} depthMaterial - The depth material.
		 * @param {Object} group - The geometry group data.
		 */
		onAfterShadow( /* renderer, object, camera, shadowCamera, geometry, depthMaterial, group */ ) {}

		/**
		 * A callback that is executed immediately before a 3D object is rendered.
		 *
		 * @param {Renderer|WebGLRenderer} renderer - The renderer.
		 * @param {Object3D} object - The 3D object.
		 * @param {Camera} camera - The camera that is used to render the scene.
		 * @param {BufferGeometry} geometry - The 3D object's geometry.
		 * @param {Material} material - The 3D object's material.
		 * @param {Object} group - The geometry group data.
		 */
		onBeforeRender( /* renderer, scene, camera, geometry, material, group */ ) {}

		/**
		 * A callback that is executed immediately after a 3D object is rendered.
		 *
		 * @param {Renderer|WebGLRenderer} renderer - The renderer.
		 * @param {Object3D} object - The 3D object.
		 * @param {Camera} camera - The camera that is used to render the scene.
		 * @param {BufferGeometry} geometry - The 3D object's geometry.
		 * @param {Material} material - The 3D object's material.
		 * @param {Object} group - The geometry group data.
		 */
		onAfterRender( /* renderer, scene, camera, geometry, material, group */ ) {}

		/**
		 * Applies the given transformation matrix to the object and updates the object's position,
		 * rotation and scale.
		 *
		 * @param {Matrix4} matrix - The transformation matrix.
		 */
		applyMatrix4( matrix ) {

			if ( this.matrixAutoUpdate ) this.updateMatrix();

			this.matrix.premultiply( matrix );

			this.matrix.decompose( this.position, this.quaternion, this.scale );

		}

		/**
		 * Applies a rotation represented by given the quaternion to the 3D object.
		 *
		 * @param {Quaternion} q - The quaternion.
		 * @return {Object3D} A reference to this instance.
		 */
		applyQuaternion( q ) {

			this.quaternion.premultiply( q );

			return this;

		}

		/**
		 * Sets the given rotation represented as an axis/angle couple to the 3D object.
		 *
		 * @param {Vector3} axis - The (normalized) axis vector.
		 * @param {number} angle - The angle in radians.
		 */
		setRotationFromAxisAngle( axis, angle ) {

			// assumes axis is normalized

			this.quaternion.setFromAxisAngle( axis, angle );

		}

		/**
		 * Sets the given rotation represented as Euler angles to the 3D object.
		 *
		 * @param {Euler} euler - The Euler angles.
		 */
		setRotationFromEuler( euler ) {

			this.quaternion.setFromEuler( euler, true );

		}

		/**
		 * Sets the given rotation represented as rotation matrix to the 3D object.
		 *
		 * @param {Matrix4} m - Although a 4x4 matrix is expected, the upper 3x3 portion must be
		 * a pure rotation matrix (i.e, unscaled).
		 */
		setRotationFromMatrix( m ) {

			// assumes the upper 3x3 of m is a pure rotation matrix (i.e, unscaled)

			this.quaternion.setFromRotationMatrix( m );

		}

		/**
		 * Sets the given rotation represented as a Quaternion to the 3D object.
		 *
		 * @param {Quaternion} q - The Quaternion
		 */
		setRotationFromQuaternion( q ) {

			// assumes q is normalized

			this.quaternion.copy( q );

		}

		/**
		 * Rotates the 3D object along an axis in local space.
		 *
		 * @param {Vector3} axis - The (normalized) axis vector.
		 * @param {number} angle - The angle in radians.
		 * @return {Object3D} A reference to this instance.
		 */
		rotateOnAxis( axis, angle ) {

			// rotate object on axis in object space
			// axis is assumed to be normalized

			_q1.setFromAxisAngle( axis, angle );

			this.quaternion.multiply( _q1 );

			return this;

		}

		/**
		 * Rotates the 3D object along an axis in world space.
		 *
		 * @param {Vector3} axis - The (normalized) axis vector.
		 * @param {number} angle - The angle in radians.
		 * @return {Object3D} A reference to this instance.
		 */
		rotateOnWorldAxis( axis, angle ) {

			// rotate object on axis in world space
			// axis is assumed to be normalized
			// method assumes no rotated parent

			_q1.setFromAxisAngle( axis, angle );

			this.quaternion.premultiply( _q1 );

			return this;

		}

		/**
		 * Rotates the 3D object around its X axis in local space.
		 *
		 * @param {number} angle - The angle in radians.
		 * @return {Object3D} A reference to this instance.
		 */
		rotateX( angle ) {

			return this.rotateOnAxis( _xAxis, angle );

		}

		/**
		 * Rotates the 3D object around its Y axis in local space.
		 *
		 * @param {number} angle - The angle in radians.
		 * @return {Object3D} A reference to this instance.
		 */
		rotateY( angle ) {

			return this.rotateOnAxis( _yAxis, angle );

		}

		/**
		 * Rotates the 3D object around its Z axis in local space.
		 *
		 * @param {number} angle - The angle in radians.
		 * @return {Object3D} A reference to this instance.
		 */
		rotateZ( angle ) {

			return this.rotateOnAxis( _zAxis, angle );

		}

		/**
		 * Translate the 3D object by a distance along the given axis in local space.
		 *
		 * @param {Vector3} axis - The (normalized) axis vector.
		 * @param {number} distance - The distance in world units.
		 * @return {Object3D} A reference to this instance.
		 */
		translateOnAxis( axis, distance ) {

			// translate object by distance along axis in object space
			// axis is assumed to be normalized

			_v1$2.copy( axis ).applyQuaternion( this.quaternion );

			this.position.add( _v1$2.multiplyScalar( distance ) );

			return this;

		}

		/**
		 * Translate the 3D object by a distance along its X-axis in local space.
		 *
		 * @param {number} distance - The distance in world units.
		 * @return {Object3D} A reference to this instance.
		 */
		translateX( distance ) {

			return this.translateOnAxis( _xAxis, distance );

		}

		/**
		 * Translate the 3D object by a distance along its Y-axis in local space.
		 *
		 * @param {number} distance - The distance in world units.
		 * @return {Object3D} A reference to this instance.
		 */
		translateY( distance ) {

			return this.translateOnAxis( _yAxis, distance );

		}

		/**
		 * Translate the 3D object by a distance along its Z-axis in local space.
		 *
		 * @param {number} distance - The distance in world units.
		 * @return {Object3D} A reference to this instance.
		 */
		translateZ( distance ) {

			return this.translateOnAxis( _zAxis, distance );

		}

		/**
		 * Converts the given vector from this 3D object's local space to world space.
		 *
		 * @param {Vector3} vector - The vector to convert.
		 * @return {Vector3} The converted vector.
		 */
		localToWorld( vector ) {

			this.updateWorldMatrix( true, false );

			return vector.applyMatrix4( this.matrixWorld );

		}

		/**
		 * Converts the given vector from this 3D object's word space to local space.
		 *
		 * @param {Vector3} vector - The vector to convert.
		 * @return {Vector3} The converted vector.
		 */
		worldToLocal( vector ) {

			this.updateWorldMatrix( true, false );

			return vector.applyMatrix4( _m1$1.copy( this.matrixWorld ).invert() );

		}

		/**
		 * Rotates the object to face a point in world space.
		 *
		 * This method does not support objects having non-uniformly-scaled parent(s).
		 *
		 * @param {number|Vector3} x - The x coordinate in world space. Alternatively, a vector representing a position in world space
		 * @param {number} [y] - The y coordinate in world space.
		 * @param {number} [z] - The z coordinate in world space.
		 */
		lookAt( x, y, z ) {

			// This method does not support objects having non-uniformly-scaled parent(s)

			if ( x.isVector3 ) {

				_target.copy( x );

			} else {

				_target.set( x, y, z );

			}

			const parent = this.parent;

			this.updateWorldMatrix( true, false );

			_position.setFromMatrixPosition( this.matrixWorld );

			if ( this.isCamera || this.isLight ) {

				_m1$1.lookAt( _position, _target, this.up );

			} else {

				_m1$1.lookAt( _target, _position, this.up );

			}

			this.quaternion.setFromRotationMatrix( _m1$1 );

			if ( parent ) {

				_m1$1.extractRotation( parent.matrixWorld );
				_q1.setFromRotationMatrix( _m1$1 );
				this.quaternion.premultiply( _q1.invert() );

			}

		}

		/**
		 * Adds the given 3D object as a child to this 3D object. An arbitrary number of
		 * objects may be added. Any current parent on an object passed in here will be
		 * removed, since an object can have at most one parent.
		 *
		 * @fires Object3D#added
		 * @fires Object3D#childadded
		 * @param {Object3D} object - The 3D object to add.
		 * @return {Object3D} A reference to this instance.
		 */
		add( object ) {

			if ( arguments.length > 1 ) {

				for ( let i = 0; i < arguments.length; i ++ ) {

					this.add( arguments[ i ] );

				}

				return this;

			}

			if ( object === this ) {

				console.error( 'THREE.Object3D.add: object can\'t be added as a child of itself.', object );
				return this;

			}

			if ( object && object.isObject3D ) {

				object.removeFromParent();
				object.parent = this;
				this.children.push( object );

				object.dispatchEvent( _addedEvent );

				_childaddedEvent.child = object;
				this.dispatchEvent( _childaddedEvent );
				_childaddedEvent.child = null;

			} else {

				console.error( 'THREE.Object3D.add: object not an instance of THREE.Object3D.', object );

			}

			return this;

		}

		/**
		 * Removes the given 3D object as child from this 3D object.
		 * An arbitrary number of objects may be removed.
		 *
		 * @fires Object3D#removed
		 * @fires Object3D#childremoved
		 * @param {Object3D} object - The 3D object to remove.
		 * @return {Object3D} A reference to this instance.
		 */
		remove( object ) {

			if ( arguments.length > 1 ) {

				for ( let i = 0; i < arguments.length; i ++ ) {

					this.remove( arguments[ i ] );

				}

				return this;

			}

			const index = this.children.indexOf( object );

			if ( index !== - 1 ) {

				object.parent = null;
				this.children.splice( index, 1 );

				object.dispatchEvent( _removedEvent );

				_childremovedEvent.child = object;
				this.dispatchEvent( _childremovedEvent );
				_childremovedEvent.child = null;

			}

			return this;

		}

		/**
		 * Removes this 3D object from its current parent.
		 *
		 * @fires Object3D#removed
		 * @fires Object3D#childremoved
		 * @return {Object3D} A reference to this instance.
		 */
		removeFromParent() {

			const parent = this.parent;

			if ( parent !== null ) {

				parent.remove( this );

			}

			return this;

		}

		/**
		 * Removes all child objects.
		 *
		 * @fires Object3D#removed
		 * @fires Object3D#childremoved
		 * @return {Object3D} A reference to this instance.
		 */
		clear() {

			return this.remove( ... this.children );

		}

		/**
		 * Adds the given 3D object as a child of this 3D object, while maintaining the object's world
		 * transform. This method does not support scene graphs having non-uniformly-scaled nodes(s).
		 *
		 * @fires Object3D#added
		 * @fires Object3D#childadded
		 * @param {Object3D} object - The 3D object to attach.
		 * @return {Object3D} A reference to this instance.
		 */
		attach( object ) {

			// adds object as a child of this, while maintaining the object's world transform

			// Note: This method does not support scene graphs having non-uniformly-scaled nodes(s)

			this.updateWorldMatrix( true, false );

			_m1$1.copy( this.matrixWorld ).invert();

			if ( object.parent !== null ) {

				object.parent.updateWorldMatrix( true, false );

				_m1$1.multiply( object.parent.matrixWorld );

			}

			object.applyMatrix4( _m1$1 );

			object.removeFromParent();
			object.parent = this;
			this.children.push( object );

			object.updateWorldMatrix( false, true );

			object.dispatchEvent( _addedEvent );

			_childaddedEvent.child = object;
			this.dispatchEvent( _childaddedEvent );
			_childaddedEvent.child = null;

			return this;

		}

		/**
		 * Searches through the 3D object and its children, starting with the 3D object
		 * itself, and returns the first with a matching ID.
		 *
		 * @param {number} id - The id.
		 * @return {Object3D|undefined} The found 3D object. Returns `undefined` if no 3D object has been found.
		 */
		getObjectById( id ) {

			return this.getObjectByProperty( 'id', id );

		}

		/**
		 * Searches through the 3D object and its children, starting with the 3D object
		 * itself, and returns the first with a matching name.
		 *
		 * @param {string} name - The name.
		 * @return {Object3D|undefined} The found 3D object. Returns `undefined` if no 3D object has been found.
		 */
		getObjectByName( name ) {

			return this.getObjectByProperty( 'name', name );

		}

		/**
		 * Searches through the 3D object and its children, starting with the 3D object
		 * itself, and returns the first with a matching property value.
		 *
		 * @param {string} name - The name of the property.
		 * @param {any} value - The value.
		 * @return {Object3D|undefined} The found 3D object. Returns `undefined` if no 3D object has been found.
		 */
		getObjectByProperty( name, value ) {

			if ( this[ name ] === value ) return this;

			for ( let i = 0, l = this.children.length; i < l; i ++ ) {

				const child = this.children[ i ];
				const object = child.getObjectByProperty( name, value );

				if ( object !== undefined ) {

					return object;

				}

			}

			return undefined;

		}

		/**
		 * Searches through the 3D object and its children, starting with the 3D object
		 * itself, and returns all 3D objects with a matching property value.
		 *
		 * @param {string} name - The name of the property.
		 * @param {any} value - The value.
		 * @param {Array<Object3D>} result - The method stores the result in this array.
		 * @return {Array<Object3D>} The found 3D objects.
		 */
		getObjectsByProperty( name, value, result = [] ) {

			if ( this[ name ] === value ) result.push( this );

			const children = this.children;

			for ( let i = 0, l = children.length; i < l; i ++ ) {

				children[ i ].getObjectsByProperty( name, value, result );

			}

			return result;

		}

		/**
		 * Returns a vector representing the position of the 3D object in world space.
		 *
		 * @param {Vector3} target - The target vector the result is stored to.
		 * @return {Vector3} The 3D object's position in world space.
		 */
		getWorldPosition( target ) {

			this.updateWorldMatrix( true, false );

			return target.setFromMatrixPosition( this.matrixWorld );

		}

		/**
		 * Returns a Quaternion representing the position of the 3D object in world space.
		 *
		 * @param {Quaternion} target - The target Quaternion the result is stored to.
		 * @return {Quaternion} The 3D object's rotation in world space.
		 */
		getWorldQuaternion( target ) {

			this.updateWorldMatrix( true, false );

			this.matrixWorld.decompose( _position, target, _scale );

			return target;

		}

		/**
		 * Returns a vector representing the scale of the 3D object in world space.
		 *
		 * @param {Vector3} target - The target vector the result is stored to.
		 * @return {Vector3} The 3D object's scale in world space.
		 */
		getWorldScale( target ) {

			this.updateWorldMatrix( true, false );

			this.matrixWorld.decompose( _position, _quaternion, target );

			return target;

		}

		/**
		 * Returns a vector representing the ("look") direction of the 3D object in world space.
		 *
		 * @param {Vector3} target - The target vector the result is stored to.
		 * @return {Vector3} The 3D object's direction in world space.
		 */
		getWorldDirection( target ) {

			this.updateWorldMatrix( true, false );

			const e = this.matrixWorld.elements;

			return target.set( e[ 8 ], e[ 9 ], e[ 10 ] ).normalize();

		}

		/**
		 * Abstract method to get intersections between a casted ray and this
		 * 3D object. Renderable 3D objects such as {@link Mesh}, {@link Line} or {@link Points}
		 * implement this method in order to use raycasting.
		 *
		 * @abstract
		 * @param {Raycaster} raycaster - The raycaster.
		 * @param {Array<Object>} intersects - An array holding the result of the method.
		 */
		raycast( /* raycaster, intersects */ ) {}

		/**
		 * Executes the callback on this 3D object and all descendants.
		 *
		 * Note: Modifying the scene graph inside the callback is discouraged.
		 *
		 * @param {Function} callback - A callback function that allows to process the current 3D object.
		 */
		traverse( callback ) {

			callback( this );

			const children = this.children;

			for ( let i = 0, l = children.length; i < l; i ++ ) {

				children[ i ].traverse( callback );

			}

		}

		/**
		 * Like {@link Object3D#traverse}, but the callback will only be executed for visible 3D objects.
		 * Descendants of invisible 3D objects are not traversed.
		 *
		 * Note: Modifying the scene graph inside the callback is discouraged.
		 *
		 * @param {Function} callback - A callback function that allows to process the current 3D object.
		 */
		traverseVisible( callback ) {

			if ( this.visible === false ) return;

			callback( this );

			const children = this.children;

			for ( let i = 0, l = children.length; i < l; i ++ ) {

				children[ i ].traverseVisible( callback );

			}

		}

		/**
		 * Like {@link Object3D#traverse}, but the callback will only be executed for all ancestors.
		 *
		 * Note: Modifying the scene graph inside the callback is discouraged.
		 *
		 * @param {Function} callback - A callback function that allows to process the current 3D object.
		 */
		traverseAncestors( callback ) {

			const parent = this.parent;

			if ( parent !== null ) {

				callback( parent );

				parent.traverseAncestors( callback );

			}

		}

		/**
		 * Updates the transformation matrix in local space by computing it from the current
		 * position, rotation and scale values.
		 */
		updateMatrix() {

			this.matrix.compose( this.position, this.quaternion, this.scale );

			this.matrixWorldNeedsUpdate = true;

		}

		/**
		 * Updates the transformation matrix in world space of this 3D objects and its descendants.
		 *
		 * To ensure correct results, this method also recomputes the 3D object's transformation matrix in
		 * local space. The computation of the local and world matrix can be controlled with the
		 * {@link Object3D#matrixAutoUpdate} and {@link Object3D#matrixWorldAutoUpdate} flags which are both
		 * `true` by default.  Set these flags to `false` if you need more control over the update matrix process.
		 *
		 * @param {boolean} [force=false] - When set to `true`, a recomputation of world matrices is forced even
		 * when {@link Object3D#matrixWorldAutoUpdate} is set to `false`.
		 */
		updateMatrixWorld( force ) {

			if ( this.matrixAutoUpdate ) this.updateMatrix();

			if ( this.matrixWorldNeedsUpdate || force ) {

				if ( this.matrixWorldAutoUpdate === true ) {

					if ( this.parent === null ) {

						this.matrixWorld.copy( this.matrix );

					} else {

						this.matrixWorld.multiplyMatrices( this.parent.matrixWorld, this.matrix );

					}

				}

				this.matrixWorldNeedsUpdate = false;

				force = true;

			}

			// make sure descendants are updated if required

			const children = this.children;

			for ( let i = 0, l = children.length; i < l; i ++ ) {

				const child = children[ i ];

				child.updateMatrixWorld( force );

			}

		}

		/**
		 * An alternative version of {@link Object3D#updateMatrixWorld} with more control over the
		 * update of ancestor and descendant nodes.
		 *
		 * @param {boolean} [updateParents=false] Whether ancestor nodes should be updated or not.
		 * @param {boolean} [updateChildren=false] Whether descendant nodes should be updated or not.
		 */
		updateWorldMatrix( updateParents, updateChildren ) {

			const parent = this.parent;

			if ( updateParents === true && parent !== null ) {

				parent.updateWorldMatrix( true, false );

			}

			if ( this.matrixAutoUpdate ) this.updateMatrix();

			if ( this.matrixWorldAutoUpdate === true ) {

				if ( this.parent === null ) {

					this.matrixWorld.copy( this.matrix );

				} else {

					this.matrixWorld.multiplyMatrices( this.parent.matrixWorld, this.matrix );

				}

			}

			// make sure descendants are updated

			if ( updateChildren === true ) {

				const children = this.children;

				for ( let i = 0, l = children.length; i < l; i ++ ) {

					const child = children[ i ];

					child.updateWorldMatrix( false, true );

				}

			}

		}

		/**
		 * Serializes the 3D object into JSON.
		 *
		 * @param {?(Object|string)} meta - An optional value holding meta information about the serialization.
		 * @return {Object} A JSON object representing the serialized 3D object.
		 * @see {@link ObjectLoader#parse}
		 */
		toJSON( meta ) {

			// meta is a string when called from JSON.stringify
			const isRootObject = ( meta === undefined || typeof meta === 'string' );

			const output = {};

			// meta is a hash used to collect geometries, materials.
			// not providing it implies that this is the root object
			// being serialized.
			if ( isRootObject ) {

				// initialize meta obj
				meta = {
					geometries: {},
					materials: {},
					textures: {},
					images: {},
					shapes: {},
					skeletons: {},
					animations: {},
					nodes: {}
				};

				output.metadata = {
					version: 4.7,
					type: 'Object',
					generator: 'Object3D.toJSON'
				};

			}

			// standard Object3D serialization

			const object = {};

			object.uuid = this.uuid;
			object.type = this.type;

			if ( this.name !== '' ) object.name = this.name;
			if ( this.castShadow === true ) object.castShadow = true;
			if ( this.receiveShadow === true ) object.receiveShadow = true;
			if ( this.visible === false ) object.visible = false;
			if ( this.frustumCulled === false ) object.frustumCulled = false;
			if ( this.renderOrder !== 0 ) object.renderOrder = this.renderOrder;
			if ( Object.keys( this.userData ).length > 0 ) object.userData = this.userData;

			object.layers = this.layers.mask;
			object.matrix = this.matrix.toArray();
			object.up = this.up.toArray();

			if ( this.matrixAutoUpdate === false ) object.matrixAutoUpdate = false;

			// object specific properties

			if ( this.isInstancedMesh ) {

				object.type = 'InstancedMesh';
				object.count = this.count;
				object.instanceMatrix = this.instanceMatrix.toJSON();
				if ( this.instanceColor !== null ) object.instanceColor = this.instanceColor.toJSON();

			}

			if ( this.isBatchedMesh ) {

				object.type = 'BatchedMesh';
				object.perObjectFrustumCulled = this.perObjectFrustumCulled;
				object.sortObjects = this.sortObjects;

				object.drawRanges = this._drawRanges;
				object.reservedRanges = this._reservedRanges;

				object.geometryInfo = this._geometryInfo.map( info => ( {
					...info,
					boundingBox: info.boundingBox ? info.boundingBox.toJSON() : undefined,
					boundingSphere: info.boundingSphere ? info.boundingSphere.toJSON() : undefined
				} ) );
				object.instanceInfo = this._instanceInfo.map( info => ( { ...info } ) );

				object.availableInstanceIds = this._availableInstanceIds.slice();
				object.availableGeometryIds = this._availableGeometryIds.slice();

				object.nextIndexStart = this._nextIndexStart;
				object.nextVertexStart = this._nextVertexStart;
				object.geometryCount = this._geometryCount;

				object.maxInstanceCount = this._maxInstanceCount;
				object.maxVertexCount = this._maxVertexCount;
				object.maxIndexCount = this._maxIndexCount;

				object.geometryInitialized = this._geometryInitialized;

				object.matricesTexture = this._matricesTexture.toJSON( meta );

				object.indirectTexture = this._indirectTexture.toJSON( meta );

				if ( this._colorsTexture !== null ) {

					object.colorsTexture = this._colorsTexture.toJSON( meta );

				}

				if ( this.boundingSphere !== null ) {

					object.boundingSphere = this.boundingSphere.toJSON();

				}

				if ( this.boundingBox !== null ) {

					object.boundingBox = this.boundingBox.toJSON();

				}

			}

			//

			function serialize( library, element ) {

				if ( library[ element.uuid ] === undefined ) {

					library[ element.uuid ] = element.toJSON( meta );

				}

				return element.uuid;

			}

			if ( this.isScene ) {

				if ( this.background ) {

					if ( this.background.isColor ) {

						object.background = this.background.toJSON();

					} else if ( this.background.isTexture ) {

						object.background = this.background.toJSON( meta ).uuid;

					}

				}

				if ( this.environment && this.environment.isTexture && this.environment.isRenderTargetTexture !== true ) {

					object.environment = this.environment.toJSON( meta ).uuid;

				}

			} else if ( this.isMesh || this.isLine || this.isPoints ) {

				object.geometry = serialize( meta.geometries, this.geometry );

				const parameters = this.geometry.parameters;

				if ( parameters !== undefined && parameters.shapes !== undefined ) {

					const shapes = parameters.shapes;

					if ( Array.isArray( shapes ) ) {

						for ( let i = 0, l = shapes.length; i < l; i ++ ) {

							const shape = shapes[ i ];

							serialize( meta.shapes, shape );

						}

					} else {

						serialize( meta.shapes, shapes );

					}

				}

			}

			if ( this.isSkinnedMesh ) {

				object.bindMode = this.bindMode;
				object.bindMatrix = this.bindMatrix.toArray();

				if ( this.skeleton !== undefined ) {

					serialize( meta.skeletons, this.skeleton );

					object.skeleton = this.skeleton.uuid;

				}

			}

			if ( this.material !== undefined ) {

				if ( Array.isArray( this.material ) ) {

					const uuids = [];

					for ( let i = 0, l = this.material.length; i < l; i ++ ) {

						uuids.push( serialize( meta.materials, this.material[ i ] ) );

					}

					object.material = uuids;

				} else {

					object.material = serialize( meta.materials, this.material );

				}

			}

			//

			if ( this.children.length > 0 ) {

				object.children = [];

				for ( let i = 0; i < this.children.length; i ++ ) {

					object.children.push( this.children[ i ].toJSON( meta ).object );

				}

			}

			//

			if ( this.animations.length > 0 ) {

				object.animations = [];

				for ( let i = 0; i < this.animations.length; i ++ ) {

					const animation = this.animations[ i ];

					object.animations.push( serialize( meta.animations, animation ) );

				}

			}

			if ( isRootObject ) {

				const geometries = extractFromCache( meta.geometries );
				const materials = extractFromCache( meta.materials );
				const textures = extractFromCache( meta.textures );
				const images = extractFromCache( meta.images );
				const shapes = extractFromCache( meta.shapes );
				const skeletons = extractFromCache( meta.skeletons );
				const animations = extractFromCache( meta.animations );
				const nodes = extractFromCache( meta.nodes );

				if ( geometries.length > 0 ) output.geometries = geometries;
				if ( materials.length > 0 ) output.materials = materials;
				if ( textures.length > 0 ) output.textures = textures;
				if ( images.length > 0 ) output.images = images;
				if ( shapes.length > 0 ) output.shapes = shapes;
				if ( skeletons.length > 0 ) output.skeletons = skeletons;
				if ( animations.length > 0 ) output.animations = animations;
				if ( nodes.length > 0 ) output.nodes = nodes;

			}

			output.object = object;

			return output;

			// extract data from the cache hash
			// remove metadata on each item
			// and return as array
			function extractFromCache( cache ) {

				const values = [];
				for ( const key in cache ) {

					const data = cache[ key ];
					delete data.metadata;
					values.push( data );

				}

				return values;

			}

		}

		/**
		 * Returns a new 3D object with copied values from this instance.
		 *
		 * @param {boolean} [recursive=true] - When set to `true`, descendants of the 3D object are also cloned.
		 * @return {Object3D} A clone of this instance.
		 */
		clone( recursive ) {

			return new this.constructor().copy( this, recursive );

		}

		/**
		 * Copies the values of the given 3D object to this instance.
		 *
		 * @param {Object3D} source - The 3D object to copy.
		 * @param {boolean} [recursive=true] - When set to `true`, descendants of the 3D object are cloned.
		 * @return {Object3D} A reference to this instance.
		 */
		copy( source, recursive = true ) {

			this.name = source.name;

			this.up.copy( source.up );

			this.position.copy( source.position );
			this.rotation.order = source.rotation.order;
			this.quaternion.copy( source.quaternion );
			this.scale.copy( source.scale );

			this.matrix.copy( source.matrix );
			this.matrixWorld.copy( source.matrixWorld );

			this.matrixAutoUpdate = source.matrixAutoUpdate;

			this.matrixWorldAutoUpdate = source.matrixWorldAutoUpdate;
			this.matrixWorldNeedsUpdate = source.matrixWorldNeedsUpdate;

			this.layers.mask = source.layers.mask;
			this.visible = source.visible;

			this.castShadow = source.castShadow;
			this.receiveShadow = source.receiveShadow;

			this.frustumCulled = source.frustumCulled;
			this.renderOrder = source.renderOrder;

			this.animations = source.animations.slice();

			this.userData = JSON.parse( JSON.stringify( source.userData ) );

			if ( recursive === true ) {

				for ( let i = 0; i < source.children.length; i ++ ) {

					const child = source.children[ i ];
					this.add( child.clone() );

				}

			}

			return this;

		}

	}

	/**
	 * The default up direction for objects, also used as the default
	 * position for {@link DirectionalLight} and {@link HemisphereLight}.
	 *
	 * @static
	 * @type {Vector3}
	 * @default (0,1,0)
	 */
	Object3D.DEFAULT_UP = /*@__PURE__*/ new Vector3( 0, 1, 0 );

	/**
	 * The default setting for {@link Object3D#matrixAutoUpdate} for
	 * newly created 3D objects.
	 *
	 * @static
	 * @type {boolean}
	 * @default true
	 */
	Object3D.DEFAULT_MATRIX_AUTO_UPDATE = true;

	/**
	 * The default setting for {@link Object3D#matrixWorldAutoUpdate} for
	 * newly created 3D objects.
	 *
	 * @static
	 * @type {boolean}
	 * @default true
	 */
	Object3D.DEFAULT_MATRIX_WORLD_AUTO_UPDATE = true;

	/**
	 * Scenes allow you to set up what is to be rendered and where by three.js.
	 * This is where you place 3D objects like meshes, lines or lights.
	 *
	 * @augments Object3D
	 */
	class Scene extends Object3D {

		/**
		 * Constructs a new scene.
		 */
		constructor() {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isScene = true;

			this.type = 'Scene';

			/**
			 * Defines the background of the scene. Valid inputs are:
			 *
			 * - A color for defining a uniform colored background.
			 * - A texture for defining a (flat) textured background.
			 * - Cube textures or equirectangular textures for defining a skybox.
			 *
			 * @type {?(Color|Texture)}
			 * @default null
			 */
			this.background = null;

			/**
			 * Sets the environment map for all physical materials in the scene. However,
			 * it's not possible to overwrite an existing texture assigned to the `envMap`
			 * material property.
			 *
			 * @type {?Texture}
			 * @default null
			 */
			this.environment = null;

			/**
			 * A fog instance defining the type of fog that affects everything
			 * rendered in the scene.
			 *
			 * @type {?(Fog|FogExp2)}
			 * @default null
			 */
			this.fog = null;

			/**
			 * Sets the blurriness of the background. Only influences environment maps
			 * assigned to {@link Scene#background}. Valid input is a float between `0`
			 * and `1`.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.backgroundBlurriness = 0;

			/**
			 * Attenuates the color of the background. Only applies to background textures.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.backgroundIntensity = 1;

			/**
			 * The rotation of the background in radians. Only influences environment maps
			 * assigned to {@link Scene#background}.
			 *
			 * @type {Euler}
			 * @default (0,0,0)
			 */
			this.backgroundRotation = new Euler();

			/**
			 * Attenuates the color of the environment. Only influences environment maps
			 * assigned to {@link Scene#environment}.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.environmentIntensity = 1;

			/**
			 * The rotation of the environment map in radians. Only influences physical materials
			 * in the scene when {@link Scene#environment} is used.
			 *
			 * @type {Euler}
			 * @default (0,0,0)
			 */
			this.environmentRotation = new Euler();

			/**
			 * Forces everything in the scene to be rendered with the defined material. It is possible
			 * to exclude materials from override by setting {@link Material#allowOverride} to `false`.
			 *
			 * @type {?Material}
			 * @default null
			 */
			this.overrideMaterial = null;

			if ( typeof __THREE_DEVTOOLS__ !== 'undefined' ) {

				__THREE_DEVTOOLS__.dispatchEvent( new CustomEvent( 'observe', { detail: this } ) );

			}

		}

		copy( source, recursive ) {

			super.copy( source, recursive );

			if ( source.background !== null ) this.background = source.background.clone();
			if ( source.environment !== null ) this.environment = source.environment.clone();
			if ( source.fog !== null ) this.fog = source.fog.clone();

			this.backgroundBlurriness = source.backgroundBlurriness;
			this.backgroundIntensity = source.backgroundIntensity;
			this.backgroundRotation.copy( source.backgroundRotation );

			this.environmentIntensity = source.environmentIntensity;
			this.environmentRotation.copy( source.environmentRotation );

			if ( source.overrideMaterial !== null ) this.overrideMaterial = source.overrideMaterial.clone();

			this.matrixAutoUpdate = source.matrixAutoUpdate;

			return this;

		}

		toJSON( meta ) {

			const data = super.toJSON( meta );

			if ( this.fog !== null ) data.object.fog = this.fog.toJSON();

			if ( this.backgroundBlurriness > 0 ) data.object.backgroundBlurriness = this.backgroundBlurriness;
			if ( this.backgroundIntensity !== 1 ) data.object.backgroundIntensity = this.backgroundIntensity;
			data.object.backgroundRotation = this.backgroundRotation.toArray();

			if ( this.environmentIntensity !== 1 ) data.object.environmentIntensity = this.environmentIntensity;
			data.object.environmentRotation = this.environmentRotation.toArray();

			return data;

		}

	}

	/**
	 * This is almost identical to an {@link Object3D}. Its purpose is to
	 * make working with groups of objects syntactically clearer.
	 *
	 * ```js
	 * // Create a group and add the two cubes.
	 * // These cubes can now be rotated / scaled etc as a group.
	 * const group = new THREE.Group();
	 *
	 * group.add( meshA );
	 * group.add( meshB );
	 *
	 * scene.add( group );
	 * ```
	 *
	 * @augments Object3D
	 */
	class Group extends Object3D {

		constructor() {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isGroup = true;

			this.type = 'Group';

		}

	}

	/**
	 * Abstract base class for cameras. This class should always be inherited
	 * when you build a new camera.
	 *
	 * @abstract
	 * @augments Object3D
	 */
	class Camera extends Object3D {

		/**
		 * Constructs a new camera.
		 */
		constructor() {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isCamera = true;

			this.type = 'Camera';

			/**
			 * The inverse of the camera's world matrix.
			 *
			 * @type {Matrix4}
			 */
			this.matrixWorldInverse = new Matrix4();

			/**
			 * The camera's projection matrix.
			 *
			 * @type {Matrix4}
			 */
			this.projectionMatrix = new Matrix4();

			/**
			 * The inverse of the camera's projection matrix.
			 *
			 * @type {Matrix4}
			 */
			this.projectionMatrixInverse = new Matrix4();

			/**
			 * The coordinate system in which the camera is used.
			 *
			 * @type {(WebGLCoordinateSystem|WebGPUCoordinateSystem)}
			 */
			this.coordinateSystem = WebGLCoordinateSystem;

			this._reversedDepth = false;

		}

		/**
		 * The flag that indicates whether the camera uses a reversed depth buffer.
		 *
		 * @type {boolean}
		 * @default false
		 */
		get reversedDepth() {

			return this._reversedDepth;

		}

		copy( source, recursive ) {

			super.copy( source, recursive );

			this.matrixWorldInverse.copy( source.matrixWorldInverse );

			this.projectionMatrix.copy( source.projectionMatrix );
			this.projectionMatrixInverse.copy( source.projectionMatrixInverse );

			this.coordinateSystem = source.coordinateSystem;

			return this;

		}

		/**
		 * Returns a vector representing the ("look") direction of the 3D object in world space.
		 *
		 * This method is overwritten since cameras have a different forward vector compared to other
		 * 3D objects. A camera looks down its local, negative z-axis by default.
		 *
		 * @param {Vector3} target - The target vector the result is stored to.
		 * @return {Vector3} The 3D object's direction in world space.
		 */
		getWorldDirection( target ) {

			return super.getWorldDirection( target ).negate();

		}

		updateMatrixWorld( force ) {

			super.updateMatrixWorld( force );

			this.matrixWorldInverse.copy( this.matrixWorld ).invert();

		}

		updateWorldMatrix( updateParents, updateChildren ) {

			super.updateWorldMatrix( updateParents, updateChildren );

			this.matrixWorldInverse.copy( this.matrixWorld ).invert();

		}

		clone() {

			return new this.constructor().copy( this );

		}

	}

	/**
	 * Class representing a 2D vector. A 2D vector is an ordered pair of numbers
	 * (labeled x and y), which can be used to represent a number of things, such as:
	 *
	 * - A point in 2D space (i.e. a position on a plane).
	 * - A direction and length across a plane. In three.js the length will
	 * always be the Euclidean distance(straight-line distance) from `(0, 0)` to `(x, y)`
	 * and the direction is also measured from `(0, 0)` towards `(x, y)`.
	 * - Any arbitrary ordered pair of numbers.
	 *
	 * There are other things a 2D vector can be used to represent, such as
	 * momentum vectors, complex numbers and so on, however these are the most
	 * common uses in three.js.
	 *
	 * Iterating through a vector instance will yield its components `(x, y)` in
	 * the corresponding order.
	 * ```js
	 * const a = new THREE.Vector2( 0, 1 );
	 *
	 * //no arguments; will be initialised to (0, 0)
	 * const b = new THREE.Vector2( );
	 *
	 * const d = a.distanceTo( b );
	 * ```
	 */
	class Vector2 {

		/**
		 * Constructs a new 2D vector.
		 *
		 * @param {number} [x=0] - The x value of this vector.
		 * @param {number} [y=0] - The y value of this vector.
		 */
		constructor( x = 0, y = 0 ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			Vector2.prototype.isVector2 = true;

			/**
			 * The x value of this vector.
			 *
			 * @type {number}
			 */
			this.x = x;

			/**
			 * The y value of this vector.
			 *
			 * @type {number}
			 */
			this.y = y;

		}

		/**
		 * Alias for {@link Vector2#x}.
		 *
		 * @type {number}
		 */
		get width() {

			return this.x;

		}

		set width( value ) {

			this.x = value;

		}

		/**
		 * Alias for {@link Vector2#y}.
		 *
		 * @type {number}
		 */
		get height() {

			return this.y;

		}

		set height( value ) {

			this.y = value;

		}

		/**
		 * Sets the vector components.
		 *
		 * @param {number} x - The value of the x component.
		 * @param {number} y - The value of the y component.
		 * @return {Vector2} A reference to this vector.
		 */
		set( x, y ) {

			this.x = x;
			this.y = y;

			return this;

		}

		/**
		 * Sets the vector components to the same value.
		 *
		 * @param {number} scalar - The value to set for all vector components.
		 * @return {Vector2} A reference to this vector.
		 */
		setScalar( scalar ) {

			this.x = scalar;
			this.y = scalar;

			return this;

		}

		/**
		 * Sets the vector's x component to the given value
		 *
		 * @param {number} x - The value to set.
		 * @return {Vector2} A reference to this vector.
		 */
		setX( x ) {

			this.x = x;

			return this;

		}

		/**
		 * Sets the vector's y component to the given value
		 *
		 * @param {number} y - The value to set.
		 * @return {Vector2} A reference to this vector.
		 */
		setY( y ) {

			this.y = y;

			return this;

		}

		/**
		 * Allows to set a vector component with an index.
		 *
		 * @param {number} index - The component index. `0` equals to x, `1` equals to y.
		 * @param {number} value - The value to set.
		 * @return {Vector2} A reference to this vector.
		 */
		setComponent( index, value ) {

			switch ( index ) {

				case 0: this.x = value; break;
				case 1: this.y = value; break;
				default: throw new Error( 'index is out of range: ' + index );

			}

			return this;

		}

		/**
		 * Returns the value of the vector component which matches the given index.
		 *
		 * @param {number} index - The component index. `0` equals to x, `1` equals to y.
		 * @return {number} A vector component value.
		 */
		getComponent( index ) {

			switch ( index ) {

				case 0: return this.x;
				case 1: return this.y;
				default: throw new Error( 'index is out of range: ' + index );

			}

		}

		/**
		 * Returns a new vector with copied values from this instance.
		 *
		 * @return {Vector2} A clone of this instance.
		 */
		clone() {

			return new this.constructor( this.x, this.y );

		}

		/**
		 * Copies the values of the given vector to this instance.
		 *
		 * @param {Vector2} v - The vector to copy.
		 * @return {Vector2} A reference to this vector.
		 */
		copy( v ) {

			this.x = v.x;
			this.y = v.y;

			return this;

		}

		/**
		 * Adds the given vector to this instance.
		 *
		 * @param {Vector2} v - The vector to add.
		 * @return {Vector2} A reference to this vector.
		 */
		add( v ) {

			this.x += v.x;
			this.y += v.y;

			return this;

		}

		/**
		 * Adds the given scalar value to all components of this instance.
		 *
		 * @param {number} s - The scalar to add.
		 * @return {Vector2} A reference to this vector.
		 */
		addScalar( s ) {

			this.x += s;
			this.y += s;

			return this;

		}

		/**
		 * Adds the given vectors and stores the result in this instance.
		 *
		 * @param {Vector2} a - The first vector.
		 * @param {Vector2} b - The second vector.
		 * @return {Vector2} A reference to this vector.
		 */
		addVectors( a, b ) {

			this.x = a.x + b.x;
			this.y = a.y + b.y;

			return this;

		}

		/**
		 * Adds the given vector scaled by the given factor to this instance.
		 *
		 * @param {Vector2} v - The vector.
		 * @param {number} s - The factor that scales `v`.
		 * @return {Vector2} A reference to this vector.
		 */
		addScaledVector( v, s ) {

			this.x += v.x * s;
			this.y += v.y * s;

			return this;

		}

		/**
		 * Subtracts the given vector from this instance.
		 *
		 * @param {Vector2} v - The vector to subtract.
		 * @return {Vector2} A reference to this vector.
		 */
		sub( v ) {

			this.x -= v.x;
			this.y -= v.y;

			return this;

		}

		/**
		 * Subtracts the given scalar value from all components of this instance.
		 *
		 * @param {number} s - The scalar to subtract.
		 * @return {Vector2} A reference to this vector.
		 */
		subScalar( s ) {

			this.x -= s;
			this.y -= s;

			return this;

		}

		/**
		 * Subtracts the given vectors and stores the result in this instance.
		 *
		 * @param {Vector2} a - The first vector.
		 * @param {Vector2} b - The second vector.
		 * @return {Vector2} A reference to this vector.
		 */
		subVectors( a, b ) {

			this.x = a.x - b.x;
			this.y = a.y - b.y;

			return this;

		}

		/**
		 * Multiplies the given vector with this instance.
		 *
		 * @param {Vector2} v - The vector to multiply.
		 * @return {Vector2} A reference to this vector.
		 */
		multiply( v ) {

			this.x *= v.x;
			this.y *= v.y;

			return this;

		}

		/**
		 * Multiplies the given scalar value with all components of this instance.
		 *
		 * @param {number} scalar - The scalar to multiply.
		 * @return {Vector2} A reference to this vector.
		 */
		multiplyScalar( scalar ) {

			this.x *= scalar;
			this.y *= scalar;

			return this;

		}

		/**
		 * Divides this instance by the given vector.
		 *
		 * @param {Vector2} v - The vector to divide.
		 * @return {Vector2} A reference to this vector.
		 */
		divide( v ) {

			this.x /= v.x;
			this.y /= v.y;

			return this;

		}

		/**
		 * Divides this vector by the given scalar.
		 *
		 * @param {number} scalar - The scalar to divide.
		 * @return {Vector2} A reference to this vector.
		 */
		divideScalar( scalar ) {

			return this.multiplyScalar( 1 / scalar );

		}

		/**
		 * Multiplies this vector (with an implicit 1 as the 3rd component) by
		 * the given 3x3 matrix.
		 *
		 * @param {Matrix3} m - The matrix to apply.
		 * @return {Vector2} A reference to this vector.
		 */
		applyMatrix3( m ) {

			const x = this.x, y = this.y;
			const e = m.elements;

			this.x = e[ 0 ] * x + e[ 3 ] * y + e[ 6 ];
			this.y = e[ 1 ] * x + e[ 4 ] * y + e[ 7 ];

			return this;

		}

		/**
		 * If this vector's x or y value is greater than the given vector's x or y
		 * value, replace that value with the corresponding min value.
		 *
		 * @param {Vector2} v - The vector.
		 * @return {Vector2} A reference to this vector.
		 */
		min( v ) {

			this.x = Math.min( this.x, v.x );
			this.y = Math.min( this.y, v.y );

			return this;

		}

		/**
		 * If this vector's x or y value is less than the given vector's x or y
		 * value, replace that value with the corresponding max value.
		 *
		 * @param {Vector2} v - The vector.
		 * @return {Vector2} A reference to this vector.
		 */
		max( v ) {

			this.x = Math.max( this.x, v.x );
			this.y = Math.max( this.y, v.y );

			return this;

		}

		/**
		 * If this vector's x or y value is greater than the max vector's x or y
		 * value, it is replaced by the corresponding value.
		 * If this vector's x or y value is less than the min vector's x or y value,
		 * it is replaced by the corresponding value.
		 *
		 * @param {Vector2} min - The minimum x and y values.
		 * @param {Vector2} max - The maximum x and y values in the desired range.
		 * @return {Vector2} A reference to this vector.
		 */
		clamp( min, max ) {

			// assumes min < max, componentwise

			this.x = clamp( this.x, min.x, max.x );
			this.y = clamp( this.y, min.y, max.y );

			return this;

		}

		/**
		 * If this vector's x or y values are greater than the max value, they are
		 * replaced by the max value.
		 * If this vector's x or y values are less than the min value, they are
		 * replaced by the min value.
		 *
		 * @param {number} minVal - The minimum value the components will be clamped to.
		 * @param {number} maxVal - The maximum value the components will be clamped to.
		 * @return {Vector2} A reference to this vector.
		 */
		clampScalar( minVal, maxVal ) {

			this.x = clamp( this.x, minVal, maxVal );
			this.y = clamp( this.y, minVal, maxVal );

			return this;

		}

		/**
		 * If this vector's length is greater than the max value, it is replaced by
		 * the max value.
		 * If this vector's length is less than the min value, it is replaced by the
		 * min value.
		 *
		 * @param {number} min - The minimum value the vector length will be clamped to.
		 * @param {number} max - The maximum value the vector length will be clamped to.
		 * @return {Vector2} A reference to this vector.
		 */
		clampLength( min, max ) {

			const length = this.length();

			return this.divideScalar( length || 1 ).multiplyScalar( clamp( length, min, max ) );

		}

		/**
		 * The components of this vector are rounded down to the nearest integer value.
		 *
		 * @return {Vector2} A reference to this vector.
		 */
		floor() {

			this.x = Math.floor( this.x );
			this.y = Math.floor( this.y );

			return this;

		}

		/**
		 * The components of this vector are rounded up to the nearest integer value.
		 *
		 * @return {Vector2} A reference to this vector.
		 */
		ceil() {

			this.x = Math.ceil( this.x );
			this.y = Math.ceil( this.y );

			return this;

		}

		/**
		 * The components of this vector are rounded to the nearest integer value
		 *
		 * @return {Vector2} A reference to this vector.
		 */
		round() {

			this.x = Math.round( this.x );
			this.y = Math.round( this.y );

			return this;

		}

		/**
		 * The components of this vector are rounded towards zero (up if negative,
		 * down if positive) to an integer value.
		 *
		 * @return {Vector2} A reference to this vector.
		 */
		roundToZero() {

			this.x = Math.trunc( this.x );
			this.y = Math.trunc( this.y );

			return this;

		}

		/**
		 * Inverts this vector - i.e. sets x = -x and y = -y.
		 *
		 * @return {Vector2} A reference to this vector.
		 */
		negate() {

			this.x = - this.x;
			this.y = - this.y;

			return this;

		}

		/**
		 * Calculates the dot product of the given vector with this instance.
		 *
		 * @param {Vector2} v - The vector to compute the dot product with.
		 * @return {number} The result of the dot product.
		 */
		dot( v ) {

			return this.x * v.x + this.y * v.y;

		}

		/**
		 * Calculates the cross product of the given vector with this instance.
		 *
		 * @param {Vector2} v - The vector to compute the cross product with.
		 * @return {number} The result of the cross product.
		 */
		cross( v ) {

			return this.x * v.y - this.y * v.x;

		}

		/**
		 * Computes the square of the Euclidean length (straight-line length) from
		 * (0, 0) to (x, y). If you are comparing the lengths of vectors, you should
		 * compare the length squared instead as it is slightly more efficient to calculate.
		 *
		 * @return {number} The square length of this vector.
		 */
		lengthSq() {

			return this.x * this.x + this.y * this.y;

		}

		/**
		 * Computes the  Euclidean length (straight-line length) from (0, 0) to (x, y).
		 *
		 * @return {number} The length of this vector.
		 */
		length() {

			return Math.sqrt( this.x * this.x + this.y * this.y );

		}

		/**
		 * Computes the Manhattan length of this vector.
		 *
		 * @return {number} The length of this vector.
		 */
		manhattanLength() {

			return Math.abs( this.x ) + Math.abs( this.y );

		}

		/**
		 * Converts this vector to a unit vector - that is, sets it equal to a vector
		 * with the same direction as this one, but with a vector length of `1`.
		 *
		 * @return {Vector2} A reference to this vector.
		 */
		normalize() {

			return this.divideScalar( this.length() || 1 );

		}

		/**
		 * Computes the angle in radians of this vector with respect to the positive x-axis.
		 *
		 * @return {number} The angle in radians.
		 */
		angle() {

			const angle = Math.atan2( - this.y, - this.x ) + Math.PI;

			return angle;

		}

		/**
		 * Returns the angle between the given vector and this instance in radians.
		 *
		 * @param {Vector2} v - The vector to compute the angle with.
		 * @return {number} The angle in radians.
		 */
		angleTo( v ) {

			const denominator = Math.sqrt( this.lengthSq() * v.lengthSq() );

			if ( denominator === 0 ) return Math.PI / 2;

			const theta = this.dot( v ) / denominator;

			// clamp, to handle numerical problems

			return Math.acos( clamp( theta, - 1, 1 ) );

		}

		/**
		 * Computes the distance from the given vector to this instance.
		 *
		 * @param {Vector2} v - The vector to compute the distance to.
		 * @return {number} The distance.
		 */
		distanceTo( v ) {

			return Math.sqrt( this.distanceToSquared( v ) );

		}

		/**
		 * Computes the squared distance from the given vector to this instance.
		 * If you are just comparing the distance with another distance, you should compare
		 * the distance squared instead as it is slightly more efficient to calculate.
		 *
		 * @param {Vector2} v - The vector to compute the squared distance to.
		 * @return {number} The squared distance.
		 */
		distanceToSquared( v ) {

			const dx = this.x - v.x, dy = this.y - v.y;
			return dx * dx + dy * dy;

		}

		/**
		 * Computes the Manhattan distance from the given vector to this instance.
		 *
		 * @param {Vector2} v - The vector to compute the Manhattan distance to.
		 * @return {number} The Manhattan distance.
		 */
		manhattanDistanceTo( v ) {

			return Math.abs( this.x - v.x ) + Math.abs( this.y - v.y );

		}

		/**
		 * Sets this vector to a vector with the same direction as this one, but
		 * with the specified length.
		 *
		 * @param {number} length - The new length of this vector.
		 * @return {Vector2} A reference to this vector.
		 */
		setLength( length ) {

			return this.normalize().multiplyScalar( length );

		}

		/**
		 * Linearly interpolates between the given vector and this instance, where
		 * alpha is the percent distance along the line - alpha = 0 will be this
		 * vector, and alpha = 1 will be the given one.
		 *
		 * @param {Vector2} v - The vector to interpolate towards.
		 * @param {number} alpha - The interpolation factor, typically in the closed interval `[0, 1]`.
		 * @return {Vector2} A reference to this vector.
		 */
		lerp( v, alpha ) {

			this.x += ( v.x - this.x ) * alpha;
			this.y += ( v.y - this.y ) * alpha;

			return this;

		}

		/**
		 * Linearly interpolates between the given vectors, where alpha is the percent
		 * distance along the line - alpha = 0 will be first vector, and alpha = 1 will
		 * be the second one. The result is stored in this instance.
		 *
		 * @param {Vector2} v1 - The first vector.
		 * @param {Vector2} v2 - The second vector.
		 * @param {number} alpha - The interpolation factor, typically in the closed interval `[0, 1]`.
		 * @return {Vector2} A reference to this vector.
		 */
		lerpVectors( v1, v2, alpha ) {

			this.x = v1.x + ( v2.x - v1.x ) * alpha;
			this.y = v1.y + ( v2.y - v1.y ) * alpha;

			return this;

		}

		/**
		 * Returns `true` if this vector is equal with the given one.
		 *
		 * @param {Vector2} v - The vector to test for equality.
		 * @return {boolean} Whether this vector is equal with the given one.
		 */
		equals( v ) {

			return ( ( v.x === this.x ) && ( v.y === this.y ) );

		}

		/**
		 * Sets this vector's x value to be `array[ offset ]` and y
		 * value to be `array[ offset + 1 ]`.
		 *
		 * @param {Array<number>} array - An array holding the vector component values.
		 * @param {number} [offset=0] - The offset into the array.
		 * @return {Vector2} A reference to this vector.
		 */
		fromArray( array, offset = 0 ) {

			this.x = array[ offset ];
			this.y = array[ offset + 1 ];

			return this;

		}

		/**
		 * Writes the components of this vector to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number>} [array=[]] - The target array holding the vector components.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number>} The vector components.
		 */
		toArray( array = [], offset = 0 ) {

			array[ offset ] = this.x;
			array[ offset + 1 ] = this.y;

			return array;

		}

		/**
		 * Sets the components of this vector from the given buffer attribute.
		 *
		 * @param {BufferAttribute} attribute - The buffer attribute holding vector data.
		 * @param {number} index - The index into the attribute.
		 * @return {Vector2} A reference to this vector.
		 */
		fromBufferAttribute( attribute, index ) {

			this.x = attribute.getX( index );
			this.y = attribute.getY( index );

			return this;

		}

		/**
		 * Rotates this vector around the given center by the given angle.
		 *
		 * @param {Vector2} center - The point around which to rotate.
		 * @param {number} angle - The angle to rotate, in radians.
		 * @return {Vector2} A reference to this vector.
		 */
		rotateAround( center, angle ) {

			const c = Math.cos( angle ), s = Math.sin( angle );

			const x = this.x - center.x;
			const y = this.y - center.y;

			this.x = x * c - y * s + center.x;
			this.y = x * s + y * c + center.y;

			return this;

		}

		/**
		 * Sets each component of this vector to a pseudo-random value between `0` and
		 * `1`, excluding `1`.
		 *
		 * @return {Vector2} A reference to this vector.
		 */
		random() {

			this.x = Math.random();
			this.y = Math.random();

			return this;

		}

		*[ Symbol.iterator ]() {

			yield this.x;
			yield this.y;

		}

	}

	const _v3$1 = /*@__PURE__*/ new Vector3();
	const _minTarget = /*@__PURE__*/ new Vector2();
	const _maxTarget = /*@__PURE__*/ new Vector2();

	/**
	 * Camera that uses [perspective projection]{@link https://en.wikipedia.org/wiki/Perspective_(graphical)}.
	 *
	 * This projection mode is designed to mimic the way the human eye sees. It
	 * is the most common projection mode used for rendering a 3D scene.
	 *
	 * ```js
	 * const camera = new THREE.PerspectiveCamera( 45, width / height, 1, 1000 );
	 * scene.add( camera );
	 * ```
	 *
	 * @augments Camera
	 */
	class PerspectiveCamera extends Camera {

		/**
		 * Constructs a new perspective camera.
		 *
		 * @param {number} [fov=50] - The vertical field of view.
		 * @param {number} [aspect=1] - The aspect ratio.
		 * @param {number} [near=0.1] - The camera's near plane.
		 * @param {number} [far=2000] - The camera's far plane.
		 */
		constructor( fov = 50, aspect = 1, near = 0.1, far = 2000 ) {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isPerspectiveCamera = true;

			this.type = 'PerspectiveCamera';

			/**
			 * The vertical field of view, from bottom to top of view,
			 * in degrees.
			 *
			 * @type {number}
			 * @default 50
			 */
			this.fov = fov;

			/**
			 * The zoom factor of the camera.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.zoom = 1;

			/**
			 * The camera's near plane. The valid range is greater than `0`
			 * and less than the current value of {@link PerspectiveCamera#far}.
			 *
			 * Note that, unlike for the {@link OrthographicCamera}, `0` is <em>not</em> a
			 * valid value for a perspective camera's near plane.
			 *
			 * @type {number}
			 * @default 0.1
			 */
			this.near = near;

			/**
			 * The camera's far plane. Must be greater than the
			 * current value of {@link PerspectiveCamera#near}.
			 *
			 * @type {number}
			 * @default 2000
			 */
			this.far = far;

			/**
			 * Object distance used for stereoscopy and depth-of-field effects. This
			 * parameter does not influence the projection matrix unless a
			 * {@link StereoCamera} is being used.
			 *
			 * @type {number}
			 * @default 10
			 */
			this.focus = 10;

			/**
			 * The aspect ratio, usually the canvas width / canvas height.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.aspect = aspect;

			/**
			 * Represents the frustum window specification. This property should not be edited
			 * directly but via {@link PerspectiveCamera#setViewOffset} and {@link PerspectiveCamera#clearViewOffset}.
			 *
			 * @type {?Object}
			 * @default null
			 */
			this.view = null;

			/**
			 * Film size used for the larger axis. Default is `35` (millimeters). This
			 * parameter does not influence the projection matrix unless {@link PerspectiveCamera#filmOffset}
			 * is set to a nonzero value.
			 *
			 * @type {number}
			 * @default 35
			 */
			this.filmGauge = 35;

			/**
			 * Horizontal off-center offset in the same unit as {@link PerspectiveCamera#filmGauge}.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.filmOffset = 0;

			this.updateProjectionMatrix();

		}

		copy( source, recursive ) {

			super.copy( source, recursive );

			this.fov = source.fov;
			this.zoom = source.zoom;

			this.near = source.near;
			this.far = source.far;
			this.focus = source.focus;

			this.aspect = source.aspect;
			this.view = source.view === null ? null : Object.assign( {}, source.view );

			this.filmGauge = source.filmGauge;
			this.filmOffset = source.filmOffset;

			return this;

		}

		/**
		 * Sets the FOV by focal length in respect to the current {@link PerspectiveCamera#filmGauge}.
		 *
		 * The default film gauge is 35, so that the focal length can be specified for
		 * a 35mm (full frame) camera.
		 *
		 * @param {number} focalLength - Values for focal length and film gauge must have the same unit.
		 */
		setFocalLength( focalLength ) {

			/** see {@link http://www.bobatkins.com/photography/technical/field_of_view.html} */
			const vExtentSlope = 0.5 * this.getFilmHeight() / focalLength;

			this.fov = RAD2DEG * 2 * Math.atan( vExtentSlope );
			this.updateProjectionMatrix();

		}

		/**
		 * Returns the focal length from the current {@link PerspectiveCamera#fov} and
		 * {@link PerspectiveCamera#filmGauge}.
		 *
		 * @return {number} The computed focal length.
		 */
		getFocalLength() {

			const vExtentSlope = Math.tan( DEG2RAD * 0.5 * this.fov );

			return 0.5 * this.getFilmHeight() / vExtentSlope;

		}

		/**
		 * Returns the current vertical field of view angle in degrees considering {@link PerspectiveCamera#zoom}.
		 *
		 * @return {number} The effective FOV.
		 */
		getEffectiveFOV() {

			return RAD2DEG * 2 * Math.atan(
				Math.tan( DEG2RAD * 0.5 * this.fov ) / this.zoom );

		}

		/**
		 * Returns the width of the image on the film. If {@link PerspectiveCamera#aspect} is greater than or
		 * equal to one (landscape format), the result equals {@link PerspectiveCamera#filmGauge}.
		 *
		 * @return {number} The film width.
		 */
		getFilmWidth() {

			// film not completely covered in portrait format (aspect < 1)
			return this.filmGauge * Math.min( this.aspect, 1 );

		}

		/**
		 * Returns the height of the image on the film. If {@link PerspectiveCamera#aspect} is greater than or
		 * equal to one (landscape format), the result equals {@link PerspectiveCamera#filmGauge}.
		 *
		 * @return {number} The film width.
		 */
		getFilmHeight() {

			// film not completely covered in landscape format (aspect > 1)
			return this.filmGauge / Math.max( this.aspect, 1 );

		}

		/**
		 * Computes the 2D bounds of the camera's viewable rectangle at a given distance along the viewing direction.
		 * Sets `minTarget` and `maxTarget` to the coordinates of the lower-left and upper-right corners of the view rectangle.
		 *
		 * @param {number} distance - The viewing distance.
		 * @param {Vector2} minTarget - The lower-left corner of the view rectangle is written into this vector.
		 * @param {Vector2} maxTarget - The upper-right corner of the view rectangle is written into this vector.
		 */
		getViewBounds( distance, minTarget, maxTarget ) {

			_v3$1.set( - 1, - 1, 0.5 ).applyMatrix4( this.projectionMatrixInverse );

			minTarget.set( _v3$1.x, _v3$1.y ).multiplyScalar( - distance / _v3$1.z );

			_v3$1.set( 1, 1, 0.5 ).applyMatrix4( this.projectionMatrixInverse );

			maxTarget.set( _v3$1.x, _v3$1.y ).multiplyScalar( - distance / _v3$1.z );

		}

		/**
		 * Computes the width and height of the camera's viewable rectangle at a given distance along the viewing direction.
		 *
		 * @param {number} distance - The viewing distance.
		 * @param {Vector2} target - The target vector that is used to store result where x is width and y is height.
		 * @returns {Vector2} The view size.
		 */
		getViewSize( distance, target ) {

			this.getViewBounds( distance, _minTarget, _maxTarget );

			return target.subVectors( _maxTarget, _minTarget );

		}

		/**
		 * Sets an offset in a larger frustum. This is useful for multi-window or
		 * multi-monitor/multi-machine setups.
		 *
		 * For example, if you have 3x2 monitors and each monitor is 1920x1080 and
		 * the monitors are in grid like this
		 *```
		 *   +---+---+---+
		 *   | A | B | C |
		 *   +---+---+---+
		 *   | D | E | F |
		 *   +---+---+---+
		 *```
		 * then for each monitor you would call it like this:
		 *```js
		 * const w = 1920;
		 * const h = 1080;
		 * const fullWidth = w * 3;
		 * const fullHeight = h * 2;
		 *
		 * // --A--
		 * camera.setViewOffset( fullWidth, fullHeight, w * 0, h * 0, w, h );
		 * // --B--
		 * camera.setViewOffset( fullWidth, fullHeight, w * 1, h * 0, w, h );
		 * // --C--
		 * camera.setViewOffset( fullWidth, fullHeight, w * 2, h * 0, w, h );
		 * // --D--
		 * camera.setViewOffset( fullWidth, fullHeight, w * 0, h * 1, w, h );
		 * // --E--
		 * camera.setViewOffset( fullWidth, fullHeight, w * 1, h * 1, w, h );
		 * // --F--
		 * camera.setViewOffset( fullWidth, fullHeight, w * 2, h * 1, w, h );
		 * ```
		 *
		 * Note there is no reason monitors have to be the same size or in a grid.
		 *
		 * @param {number} fullWidth - The full width of multiview setup.
		 * @param {number} fullHeight - The full height of multiview setup.
		 * @param {number} x - The horizontal offset of the subcamera.
		 * @param {number} y - The vertical offset of the subcamera.
		 * @param {number} width - The width of subcamera.
		 * @param {number} height - The height of subcamera.
		 */
		setViewOffset( fullWidth, fullHeight, x, y, width, height ) {

			this.aspect = fullWidth / fullHeight;

			if ( this.view === null ) {

				this.view = {
					enabled: true,
					fullWidth: 1,
					fullHeight: 1,
					offsetX: 0,
					offsetY: 0,
					width: 1,
					height: 1
				};

			}

			this.view.enabled = true;
			this.view.fullWidth = fullWidth;
			this.view.fullHeight = fullHeight;
			this.view.offsetX = x;
			this.view.offsetY = y;
			this.view.width = width;
			this.view.height = height;

			this.updateProjectionMatrix();

		}

		/**
		 * Removes the view offset from the projection matrix.
		 */
		clearViewOffset() {

			if ( this.view !== null ) {

				this.view.enabled = false;

			}

			this.updateProjectionMatrix();

		}

		/**
		 * Updates the camera's projection matrix. Must be called after any change of
		 * camera properties.
		 */
		updateProjectionMatrix() {

			const near = this.near;
			let top = near * Math.tan( DEG2RAD * 0.5 * this.fov ) / this.zoom;
			let height = 2 * top;
			let width = this.aspect * height;
			let left = - 0.5 * width;
			const view = this.view;

			if ( this.view !== null && this.view.enabled ) {

				const fullWidth = view.fullWidth,
					fullHeight = view.fullHeight;

				left += view.offsetX * width / fullWidth;
				top -= view.offsetY * height / fullHeight;
				width *= view.width / fullWidth;
				height *= view.height / fullHeight;

			}

			const skew = this.filmOffset;
			if ( skew !== 0 ) left += near * skew / this.getFilmWidth();

			this.projectionMatrix.makePerspective( left, left + width, top, top - height, near, this.far, this.coordinateSystem, this.reversedDepth );

			this.projectionMatrixInverse.copy( this.projectionMatrix ).invert();

		}

		toJSON( meta ) {

			const data = super.toJSON( meta );

			data.object.fov = this.fov;
			data.object.zoom = this.zoom;

			data.object.near = this.near;
			data.object.far = this.far;
			data.object.focus = this.focus;

			data.object.aspect = this.aspect;

			if ( this.view !== null ) data.object.view = Object.assign( {}, this.view );

			data.object.filmGauge = this.filmGauge;
			data.object.filmOffset = this.filmOffset;

			return data;

		}

	}

	const _vector$2 = /*@__PURE__*/ new Vector3();
	const _vector2 = /*@__PURE__*/ new Vector2();

	let _id$1 = 0;

	/**
	 * This class stores data for an attribute (such as vertex positions, face
	 * indices, normals, colors, UVs, and any custom attributes ) associated with
	 * a geometry, which allows for more efficient passing of data to the GPU.
	 *
	 * When working with vector-like data, the `fromBufferAttribute( attribute, index )`
	 * helper methods on vector and color class might be helpful. E.g. {@link Vector3#fromBufferAttribute}.
	 */
	class BufferAttribute {

		/**
		 * Constructs a new buffer attribute.
		 *
		 * @param {TypedArray} array - The array holding the attribute data.
		 * @param {number} itemSize - The item size.
		 * @param {boolean} [normalized=false] - Whether the data are normalized or not.
		 */
		constructor( array, itemSize, normalized = false ) {

			if ( Array.isArray( array ) ) {

				throw new TypeError( 'THREE.BufferAttribute: array should be a Typed Array.' );

			}

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isBufferAttribute = true;

			/**
			 * The ID of the buffer attribute.
			 *
			 * @name BufferAttribute#id
			 * @type {number}
			 * @readonly
			 */
			Object.defineProperty( this, 'id', { value: _id$1 ++ } );

			/**
			 * The name of the buffer attribute.
			 *
			 * @type {string}
			 */
			this.name = '';

			/**
			 * The array holding the attribute data. It should have `itemSize * numVertices`
			 * elements, where `numVertices` is the number of vertices in the associated geometry.
			 *
			 * @type {TypedArray}
			 */
			this.array = array;

			/**
			 * The number of values of the array that should be associated with a particular vertex.
			 * For instance, if this attribute is storing a 3-component vector (such as a position,
			 * normal, or color), then the value should be `3`.
			 *
			 * @type {number}
			 */
			this.itemSize = itemSize;

			/**
			 * Represents the number of items this buffer attribute stores. It is internally computed
			 * by dividing the `array` length by the `itemSize`.
			 *
			 * @type {number}
			 * @readonly
			 */
			this.count = array !== undefined ? array.length / itemSize : 0;

			/**
			 * Applies to integer data only. Indicates how the underlying data in the buffer maps to
			 * the values in the GLSL code. For instance, if `array` is an instance of `UInt16Array`,
			 * and `normalized` is `true`, the values `0 - +65535` in the array data will be mapped to
			 * `0.0f - +1.0f` in the GLSL attribute. If `normalized` is `false`, the values will be converted
			 * to floats unmodified, i.e. `65535` becomes `65535.0f`.
			 *
			 * @type {boolean}
			 */
			this.normalized = normalized;

			/**
			 * Defines the intended usage pattern of the data store for optimization purposes.
			 *
			 * Note: After the initial use of a buffer, its usage cannot be changed. Instead,
			 * instantiate a new one and set the desired usage before the next render.
			 *
			 * @type {(StaticDrawUsage|DynamicDrawUsage|StreamDrawUsage|StaticReadUsage|DynamicReadUsage|StreamReadUsage|StaticCopyUsage|DynamicCopyUsage|StreamCopyUsage)}
			 * @default StaticDrawUsage
			 */
			this.usage = StaticDrawUsage;

			/**
			 * This can be used to only update some components of stored vectors (for example, just the
			 * component related to color). Use the `addUpdateRange()` function to add ranges to this array.
			 *
			 * @type {Array<Object>}
			 */
			this.updateRanges = [];

			/**
			 * Configures the bound GPU type for use in shaders.
			 *
			 * Note: this only has an effect for integer arrays and is not configurable for float arrays.
			 * For lower precision float types, use `Float16BufferAttribute`.
			 *
			 * @type {(FloatType|IntType)}
			 * @default FloatType
			 */
			this.gpuType = FloatType;

			/**
			 * A version number, incremented every time the `needsUpdate` is set to `true`.
			 *
			 * @type {number}
			 */
			this.version = 0;

		}

		/**
		 * A callback function that is executed after the renderer has transferred the attribute
		 * array data to the GPU.
		 */
		onUploadCallback() {}

		/**
		 * Flag to indicate that this attribute has changed and should be re-sent to
		 * the GPU. Set this to `true` when you modify the value of the array.
		 *
		 * @type {number}
		 * @default false
		 * @param {boolean} value
		 */
		set needsUpdate( value ) {

			if ( value === true ) this.version ++;

		}

		/**
		 * Sets the usage of this buffer attribute.
		 *
		 * @param {(StaticDrawUsage|DynamicDrawUsage|StreamDrawUsage|StaticReadUsage|DynamicReadUsage|StreamReadUsage|StaticCopyUsage|DynamicCopyUsage|StreamCopyUsage)} value - The usage to set.
		 * @return {BufferAttribute} A reference to this buffer attribute.
		 */
		setUsage( value ) {

			this.usage = value;

			return this;

		}

		/**
		 * Adds a range of data in the data array to be updated on the GPU.
		 *
		 * @param {number} start - Position at which to start update.
		 * @param {number} count - The number of components to update.
		 */
		addUpdateRange( start, count ) {

			this.updateRanges.push( { start, count } );

		}

		/**
		 * Clears the update ranges.
		 */
		clearUpdateRanges() {

			this.updateRanges.length = 0;

		}

		/**
		 * Copies the values of the given buffer attribute to this instance.
		 *
		 * @param {BufferAttribute} source - The buffer attribute to copy.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		copy( source ) {

			this.name = source.name;
			this.array = new source.array.constructor( source.array );
			this.itemSize = source.itemSize;
			this.count = source.count;
			this.normalized = source.normalized;

			this.usage = source.usage;
			this.gpuType = source.gpuType;

			return this;

		}

		/**
		 * Copies a vector from the given buffer attribute to this one. The start
		 * and destination position in the attribute buffers are represented by the
		 * given indices.
		 *
		 * @param {number} index1 - The destination index into this buffer attribute.
		 * @param {BufferAttribute} attribute - The buffer attribute to copy from.
		 * @param {number} index2 - The source index into the given buffer attribute.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		copyAt( index1, attribute, index2 ) {

			index1 *= this.itemSize;
			index2 *= attribute.itemSize;

			for ( let i = 0, l = this.itemSize; i < l; i ++ ) {

				this.array[ index1 + i ] = attribute.array[ index2 + i ];

			}

			return this;

		}

		/**
		 * Copies the given array data into this buffer attribute.
		 *
		 * @param {(TypedArray|Array)} array - The array to copy.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		copyArray( array ) {

			this.array.set( array );

			return this;

		}

		/**
		 * Applies the given 3x3 matrix to the given attribute. Works with
		 * item size `2` and `3`.
		 *
		 * @param {Matrix3} m - The matrix to apply.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		applyMatrix3( m ) {

			if ( this.itemSize === 2 ) {

				for ( let i = 0, l = this.count; i < l; i ++ ) {

					_vector2.fromBufferAttribute( this, i );
					_vector2.applyMatrix3( m );

					this.setXY( i, _vector2.x, _vector2.y );

				}

			} else if ( this.itemSize === 3 ) {

				for ( let i = 0, l = this.count; i < l; i ++ ) {

					_vector$2.fromBufferAttribute( this, i );
					_vector$2.applyMatrix3( m );

					this.setXYZ( i, _vector$2.x, _vector$2.y, _vector$2.z );

				}

			}

			return this;

		}

		/**
		 * Applies the given 4x4 matrix to the given attribute. Only works with
		 * item size `3`.
		 *
		 * @param {Matrix4} m - The matrix to apply.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		applyMatrix4( m ) {

			for ( let i = 0, l = this.count; i < l; i ++ ) {

				_vector$2.fromBufferAttribute( this, i );

				_vector$2.applyMatrix4( m );

				this.setXYZ( i, _vector$2.x, _vector$2.y, _vector$2.z );

			}

			return this;

		}

		/**
		 * Applies the given 3x3 normal matrix to the given attribute. Only works with
		 * item size `3`.
		 *
		 * @param {Matrix3} m - The normal matrix to apply.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		applyNormalMatrix( m ) {

			for ( let i = 0, l = this.count; i < l; i ++ ) {

				_vector$2.fromBufferAttribute( this, i );

				_vector$2.applyNormalMatrix( m );

				this.setXYZ( i, _vector$2.x, _vector$2.y, _vector$2.z );

			}

			return this;

		}

		/**
		 * Applies the given 4x4 matrix to the given attribute. Only works with
		 * item size `3` and with direction vectors.
		 *
		 * @param {Matrix4} m - The matrix to apply.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		transformDirection( m ) {

			for ( let i = 0, l = this.count; i < l; i ++ ) {

				_vector$2.fromBufferAttribute( this, i );

				_vector$2.transformDirection( m );

				this.setXYZ( i, _vector$2.x, _vector$2.y, _vector$2.z );

			}

			return this;

		}

		/**
		 * Sets the given array data in the buffer attribute.
		 *
		 * @param {(TypedArray|Array)} value - The array data to set.
		 * @param {number} [offset=0] - The offset in this buffer attribute's array.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		set( value, offset = 0 ) {

			// Matching BufferAttribute constructor, do not normalize the array.
			this.array.set( value, offset );

			return this;

		}

		/**
		 * Returns the given component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} component - The component index.
		 * @return {number} The returned value.
		 */
		getComponent( index, component ) {

			let value = this.array[ index * this.itemSize + component ];

			if ( this.normalized ) value = denormalize( value, this.array );

			return value;

		}

		/**
		 * Sets the given value to the given component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} component - The component index.
		 * @param {number} value - The value to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setComponent( index, component, value ) {

			if ( this.normalized ) value = normalize( value, this.array );

			this.array[ index * this.itemSize + component ] = value;

			return this;

		}

		/**
		 * Returns the x component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @return {number} The x component.
		 */
		getX( index ) {

			let x = this.array[ index * this.itemSize ];

			if ( this.normalized ) x = denormalize( x, this.array );

			return x;

		}

		/**
		 * Sets the x component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} x - The value to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setX( index, x ) {

			if ( this.normalized ) x = normalize( x, this.array );

			this.array[ index * this.itemSize ] = x;

			return this;

		}

		/**
		 * Returns the y component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @return {number} The y component.
		 */
		getY( index ) {

			let y = this.array[ index * this.itemSize + 1 ];

			if ( this.normalized ) y = denormalize( y, this.array );

			return y;

		}

		/**
		 * Sets the y component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} y - The value to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setY( index, y ) {

			if ( this.normalized ) y = normalize( y, this.array );

			this.array[ index * this.itemSize + 1 ] = y;

			return this;

		}

		/**
		 * Returns the z component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @return {number} The z component.
		 */
		getZ( index ) {

			let z = this.array[ index * this.itemSize + 2 ];

			if ( this.normalized ) z = denormalize( z, this.array );

			return z;

		}

		/**
		 * Sets the z component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} z - The value to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setZ( index, z ) {

			if ( this.normalized ) z = normalize( z, this.array );

			this.array[ index * this.itemSize + 2 ] = z;

			return this;

		}

		/**
		 * Returns the w component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @return {number} The w component.
		 */
		getW( index ) {

			let w = this.array[ index * this.itemSize + 3 ];

			if ( this.normalized ) w = denormalize( w, this.array );

			return w;

		}

		/**
		 * Sets the w component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} w - The value to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setW( index, w ) {

			if ( this.normalized ) w = normalize( w, this.array );

			this.array[ index * this.itemSize + 3 ] = w;

			return this;

		}

		/**
		 * Sets the x and y component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} x - The value for the x component to set.
		 * @param {number} y - The value for the y component to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setXY( index, x, y ) {

			index *= this.itemSize;

			if ( this.normalized ) {

				x = normalize( x, this.array );
				y = normalize( y, this.array );

			}

			this.array[ index + 0 ] = x;
			this.array[ index + 1 ] = y;

			return this;

		}

		/**
		 * Sets the x, y and z component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} x - The value for the x component to set.
		 * @param {number} y - The value for the y component to set.
		 * @param {number} z - The value for the z component to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setXYZ( index, x, y, z ) {

			index *= this.itemSize;

			if ( this.normalized ) {

				x = normalize( x, this.array );
				y = normalize( y, this.array );
				z = normalize( z, this.array );

			}

			this.array[ index + 0 ] = x;
			this.array[ index + 1 ] = y;
			this.array[ index + 2 ] = z;

			return this;

		}

		/**
		 * Sets the x, y, z and w component of the vector at the given index.
		 *
		 * @param {number} index - The index into the buffer attribute.
		 * @param {number} x - The value for the x component to set.
		 * @param {number} y - The value for the y component to set.
		 * @param {number} z - The value for the z component to set.
		 * @param {number} w - The value for the w component to set.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		setXYZW( index, x, y, z, w ) {

			index *= this.itemSize;

			if ( this.normalized ) {

				x = normalize( x, this.array );
				y = normalize( y, this.array );
				z = normalize( z, this.array );
				w = normalize( w, this.array );

			}

			this.array[ index + 0 ] = x;
			this.array[ index + 1 ] = y;
			this.array[ index + 2 ] = z;
			this.array[ index + 3 ] = w;

			return this;

		}

		/**
		 * Sets the given callback function that is executed after the Renderer has transferred
		 * the attribute array data to the GPU. Can be used to perform clean-up operations after
		 * the upload when attribute data are not needed anymore on the CPU side.
		 *
		 * @param {Function} callback - The `onUpload()` callback.
		 * @return {BufferAttribute} A reference to this instance.
		 */
		onUpload( callback ) {

			this.onUploadCallback = callback;

			return this;

		}

		/**
		 * Returns a new buffer attribute with copied values from this instance.
		 *
		 * @return {BufferAttribute} A clone of this instance.
		 */
		clone() {

			return new this.constructor( this.array, this.itemSize ).copy( this );

		}

		/**
		 * Serializes the buffer attribute into JSON.
		 *
		 * @return {Object} A JSON object representing the serialized buffer attribute.
		 */
		toJSON() {

			const data = {
				itemSize: this.itemSize,
				type: this.array.constructor.name,
				array: Array.from( this.array ),
				normalized: this.normalized
			};

			if ( this.name !== '' ) data.name = this.name;
			if ( this.usage !== StaticDrawUsage ) data.usage = this.usage;

			return data;

		}

	}

	/**
	 * Convenient class that can be used when creating a `UInt16` buffer attribute with
	 * a plain `Array` instance.
	 *
	 * @augments BufferAttribute
	 */
	class Uint16BufferAttribute extends BufferAttribute {

		/**
		 * Constructs a new buffer attribute.
		 *
		 * @param {(Array<number>|Uint16Array)} array - The array holding the attribute data.
		 * @param {number} itemSize - The item size.
		 * @param {boolean} [normalized=false] - Whether the data are normalized or not.
		 */
		constructor( array, itemSize, normalized ) {

			super( new Uint16Array( array ), itemSize, normalized );

		}

	}

	/**
	 * Convenient class that can be used when creating a `UInt32` buffer attribute with
	 * a plain `Array` instance.
	 *
	 * @augments BufferAttribute
	 */
	class Uint32BufferAttribute extends BufferAttribute {

		/**
		 * Constructs a new buffer attribute.
		 *
		 * @param {(Array<number>|Uint32Array)} array - The array holding the attribute data.
		 * @param {number} itemSize - The item size.
		 * @param {boolean} [normalized=false] - Whether the data are normalized or not.
		 */
		constructor( array, itemSize, normalized ) {

			super( new Uint32Array( array ), itemSize, normalized );

		}

	}

	/**
	 * Convenient class that can be used when creating a `Float32` buffer attribute with
	 * a plain `Array` instance.
	 *
	 * @augments BufferAttribute
	 */
	class Float32BufferAttribute extends BufferAttribute {

		/**
		 * Constructs a new buffer attribute.
		 *
		 * @param {(Array<number>|Float32Array)} array - The array holding the attribute data.
		 * @param {number} itemSize - The item size.
		 * @param {boolean} [normalized=false] - Whether the data are normalized or not.
		 */
		constructor( array, itemSize, normalized ) {

			super( new Float32Array( array ), itemSize, normalized );

		}

	}

	const _box$1 = /*@__PURE__*/ new Box3();
	const _v1$1 = /*@__PURE__*/ new Vector3();
	const _v2$1 = /*@__PURE__*/ new Vector3();

	/**
	 * An analytical 3D sphere defined by a center and radius. This class is mainly
	 * used as a Bounding Sphere for 3D objects.
	 */
	class Sphere {

		/**
		 * Constructs a new sphere.
		 *
		 * @param {Vector3} [center=(0,0,0)] - The center of the sphere
		 * @param {number} [radius=-1] - The radius of the sphere.
		 */
		constructor( center = new Vector3(), radius = - 1 ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isSphere = true;

			/**
			 * The center of the sphere
			 *
			 * @type {Vector3}
			 */
			this.center = center;

			/**
			 * The radius of the sphere.
			 *
			 * @type {number}
			 */
			this.radius = radius;

		}

		/**
		 * Sets the sphere's components by copying the given values.
		 *
		 * @param {Vector3} center - The center.
		 * @param {number} radius - The radius.
		 * @return {Sphere} A reference to this sphere.
		 */
		set( center, radius ) {

			this.center.copy( center );
			this.radius = radius;

			return this;

		}

		/**
		 * Computes the minimum bounding sphere for list of points.
		 * If the optional center point is given, it is used as the sphere's
		 * center. Otherwise, the center of the axis-aligned bounding box
		 * encompassing the points is calculated.
		 *
		 * @param {Array<Vector3>} points - A list of points in 3D space.
		 * @param {Vector3} [optionalCenter] - The center of the sphere.
		 * @return {Sphere} A reference to this sphere.
		 */
		setFromPoints( points, optionalCenter ) {

			const center = this.center;

			if ( optionalCenter !== undefined ) {

				center.copy( optionalCenter );

			} else {

				_box$1.setFromPoints( points ).getCenter( center );

			}

			let maxRadiusSq = 0;

			for ( let i = 0, il = points.length; i < il; i ++ ) {

				maxRadiusSq = Math.max( maxRadiusSq, center.distanceToSquared( points[ i ] ) );

			}

			this.radius = Math.sqrt( maxRadiusSq );

			return this;

		}

		/**
		 * Copies the values of the given sphere to this instance.
		 *
		 * @param {Sphere} sphere - The sphere to copy.
		 * @return {Sphere} A reference to this sphere.
		 */
		copy( sphere ) {

			this.center.copy( sphere.center );
			this.radius = sphere.radius;

			return this;

		}

		/**
		 * Returns `true` if the sphere is empty (the radius set to a negative number).
		 *
		 * Spheres with a radius of `0` contain only their center point and are not
		 * considered to be empty.
		 *
		 * @return {boolean} Whether this sphere is empty or not.
		 */
		isEmpty() {

			return ( this.radius < 0 );

		}

		/**
		 * Makes this sphere empty which means in encloses a zero space in 3D.
		 *
		 * @return {Sphere} A reference to this sphere.
		 */
		makeEmpty() {

			this.center.set( 0, 0, 0 );
			this.radius = - 1;

			return this;

		}

		/**
		 * Returns `true` if this sphere contains the given point inclusive of
		 * the surface of the sphere.
		 *
		 * @param {Vector3} point - The point to check.
		 * @return {boolean} Whether this sphere contains the given point or not.
		 */
		containsPoint( point ) {

			return ( point.distanceToSquared( this.center ) <= ( this.radius * this.radius ) );

		}

		/**
		 * Returns the closest distance from the boundary of the sphere to the
		 * given point. If the sphere contains the point, the distance will
		 * be negative.
		 *
		 * @param {Vector3} point - The point to compute the distance to.
		 * @return {number} The distance to the point.
		 */
		distanceToPoint( point ) {

			return ( point.distanceTo( this.center ) - this.radius );

		}

		/**
		 * Returns `true` if this sphere intersects with the given one.
		 *
		 * @param {Sphere} sphere - The sphere to test.
		 * @return {boolean} Whether this sphere intersects with the given one or not.
		 */
		intersectsSphere( sphere ) {

			const radiusSum = this.radius + sphere.radius;

			return sphere.center.distanceToSquared( this.center ) <= ( radiusSum * radiusSum );

		}

		/**
		 * Returns `true` if this sphere intersects with the given box.
		 *
		 * @param {Box3} box - The box to test.
		 * @return {boolean} Whether this sphere intersects with the given box or not.
		 */
		intersectsBox( box ) {

			return box.intersectsSphere( this );

		}

		/**
		 * Returns `true` if this sphere intersects with the given plane.
		 *
		 * @param {Plane} plane - The plane to test.
		 * @return {boolean} Whether this sphere intersects with the given plane or not.
		 */
		intersectsPlane( plane ) {

			return Math.abs( plane.distanceToPoint( this.center ) ) <= this.radius;

		}

		/**
		 * Clamps a point within the sphere. If the point is outside the sphere, it
		 * will clamp it to the closest point on the edge of the sphere. Points
		 * already inside the sphere will not be affected.
		 *
		 * @param {Vector3} point - The plane to clamp.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The clamped point.
		 */
		clampPoint( point, target ) {

			const deltaLengthSq = this.center.distanceToSquared( point );

			target.copy( point );

			if ( deltaLengthSq > ( this.radius * this.radius ) ) {

				target.sub( this.center ).normalize();
				target.multiplyScalar( this.radius ).add( this.center );

			}

			return target;

		}

		/**
		 * Returns a bounding box that encloses this sphere.
		 *
		 * @param {Box3} target - The target box that is used to store the method's result.
		 * @return {Box3} The bounding box that encloses this sphere.
		 */
		getBoundingBox( target ) {

			if ( this.isEmpty() ) {

				// Empty sphere produces empty bounding box
				target.makeEmpty();
				return target;

			}

			target.set( this.center, this.center );
			target.expandByScalar( this.radius );

			return target;

		}

		/**
		 * Transforms this sphere with the given 4x4 transformation matrix.
		 *
		 * @param {Matrix4} matrix - The transformation matrix.
		 * @return {Sphere} A reference to this sphere.
		 */
		applyMatrix4( matrix ) {

			this.center.applyMatrix4( matrix );
			this.radius = this.radius * matrix.getMaxScaleOnAxis();

			return this;

		}

		/**
		 * Translates the sphere's center by the given offset.
		 *
		 * @param {Vector3} offset - The offset.
		 * @return {Sphere} A reference to this sphere.
		 */
		translate( offset ) {

			this.center.add( offset );

			return this;

		}

		/**
		 * Expands the boundaries of this sphere to include the given point.
		 *
		 * @param {Vector3} point - The point to include.
		 * @return {Sphere} A reference to this sphere.
		 */
		expandByPoint( point ) {

			if ( this.isEmpty() ) {

				this.center.copy( point );

				this.radius = 0;

				return this;

			}

			_v1$1.subVectors( point, this.center );

			const lengthSq = _v1$1.lengthSq();

			if ( lengthSq > ( this.radius * this.radius ) ) {

				// calculate the minimal sphere

				const length = Math.sqrt( lengthSq );

				const delta = ( length - this.radius ) * 0.5;

				this.center.addScaledVector( _v1$1, delta / length );

				this.radius += delta;

			}

			return this;

		}

		/**
		 * Expands this sphere to enclose both the original sphere and the given sphere.
		 *
		 * @param {Sphere} sphere - The sphere to include.
		 * @return {Sphere} A reference to this sphere.
		 */
		union( sphere ) {

			if ( sphere.isEmpty() ) {

				return this;

			}

			if ( this.isEmpty() ) {

				this.copy( sphere );

				return this;

			}

			if ( this.center.equals( sphere.center ) === true ) {

				 this.radius = Math.max( this.radius, sphere.radius );

			} else {

				_v2$1.subVectors( sphere.center, this.center ).setLength( sphere.radius );

				this.expandByPoint( _v1$1.copy( sphere.center ).add( _v2$1 ) );

				this.expandByPoint( _v1$1.copy( sphere.center ).sub( _v2$1 ) );

			}

			return this;

		}

		/**
		 * Returns `true` if this sphere is equal with the given one.
		 *
		 * @param {Sphere} sphere - The sphere to test for equality.
		 * @return {boolean} Whether this bounding sphere is equal with the given one.
		 */
		equals( sphere ) {

			return sphere.center.equals( this.center ) && ( sphere.radius === this.radius );

		}

		/**
		 * Returns a new sphere with copied values from this instance.
		 *
		 * @return {Sphere} A clone of this instance.
		 */
		clone() {

			return new this.constructor().copy( this );

		}

		/**
		 * Returns a serialized structure of the bounding sphere.
		 *
		 * @return {Object} Serialized structure with fields representing the object state.
		 */
		toJSON() {

			return {
				radius: this.radius,
				center: this.center.toArray()
			};

		}

		/**
		 * Returns a serialized structure of the bounding sphere.
		 *
		 * @param {Object} json - The serialized json to set the sphere from.
		 * @return {Box3} A reference to this bounding sphere.
		 */
		fromJSON( json ) {

			this.radius = json.radius;
			this.center.fromArray( json.center );
			return this;

		}

	}

	let _id = 0;

	const _m1 = /*@__PURE__*/ new Matrix4();
	const _obj = /*@__PURE__*/ new Object3D();
	const _offset = /*@__PURE__*/ new Vector3();
	const _box = /*@__PURE__*/ new Box3();
	const _boxMorphTargets = /*@__PURE__*/ new Box3();
	const _vector$1 = /*@__PURE__*/ new Vector3();

	/**
	 * A representation of mesh, line, or point geometry. Includes vertex
	 * positions, face indices, normals, colors, UVs, and custom attributes
	 * within buffers, reducing the cost of passing all this data to the GPU.
	 *
	 * ```js
	 * const geometry = new THREE.BufferGeometry();
	 * // create a simple square shape. We duplicate the top left and bottom right
	 * // vertices because each vertex needs to appear once per triangle.
	 * const vertices = new Float32Array( [
	 * 	-1.0, -1.0,  1.0, // v0
	 * 	 1.0, -1.0,  1.0, // v1
	 * 	 1.0,  1.0,  1.0, // v2
	 *
	 * 	 1.0,  1.0,  1.0, // v3
	 * 	-1.0,  1.0,  1.0, // v4
	 * 	-1.0, -1.0,  1.0  // v5
	 * ] );
	 * // itemSize = 3 because there are 3 values (components) per vertex
	 * geometry.setAttribute( 'position', new THREE.BufferAttribute( vertices, 3 ) );
	 * const material = new THREE.MeshBasicMaterial( { color: 0xff0000 } );
	 * const mesh = new THREE.Mesh( geometry, material );
	 * ```
	 *
	 * @augments EventDispatcher
	 */
	class BufferGeometry extends EventDispatcher {

		/**
		 * Constructs a new geometry.
		 */
		constructor() {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isBufferGeometry = true;

			/**
			 * The ID of the geometry.
			 *
			 * @name BufferGeometry#id
			 * @type {number}
			 * @readonly
			 */
			Object.defineProperty( this, 'id', { value: _id ++ } );

			/**
			 * The UUID of the geometry.
			 *
			 * @type {string}
			 * @readonly
			 */
			this.uuid = generateUUID();

			/**
			 * The name of the geometry.
			 *
			 * @type {string}
			 */
			this.name = '';
			this.type = 'BufferGeometry';

			/**
			 * Allows for vertices to be re-used across multiple triangles; this is
			 * called using "indexed triangles". Each triangle is associated with the
			 * indices of three vertices. This attribute therefore stores the index of
			 * each vertex for each triangular face. If this attribute is not set, the
			 * renderer assumes that each three contiguous positions represent a single triangle.
			 *
			 * @type {?BufferAttribute}
			 * @default null
			 */
			this.index = null;

			/**
			 * A (storage) buffer attribute which was generated with a compute shader and
			 * now defines indirect draw calls.
			 *
			 * Can only be used with {@link WebGPURenderer} and a WebGPU backend.
			 *
			 * @type {?BufferAttribute}
			 * @default null
			 */
			this.indirect = null;

			/**
			 * This dictionary has as id the name of the attribute to be set and as value
			 * the buffer attribute to set it to. Rather than accessing this property directly,
			 * use `setAttribute()` and `getAttribute()` to access attributes of this geometry.
			 *
			 * @type {Object<string,(BufferAttribute|InterleavedBufferAttribute)>}
			 */
			this.attributes = {};

			/**
			 * This dictionary holds the morph targets of the geometry.
			 *
			 * Note: Once the geometry has been rendered, the morph attribute data cannot
			 * be changed. You will have to call `dispose()?, and create a new geometry instance.
			 *
			 * @type {Object}
			 */
			this.morphAttributes = {};

			/**
			 * Used to control the morph target behavior; when set to `true`, the morph
			 * target data is treated as relative offsets, rather than as absolute
			 * positions/normals.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.morphTargetsRelative = false;

			/**
			 * Split the geometry into groups, each of which will be rendered in a
			 * separate draw call. This allows an array of materials to be used with the geometry.
			 *
			 * Use `addGroup()` and `clearGroups()` to edit groups, rather than modifying this array directly.
			 *
			 * Every vertex and index must belong to exactly one group — groups must not share vertices or
			 * indices, and must not leave vertices or indices unused.
			 *
			 * @type {Array<Object>}
			 */
			this.groups = [];

			/**
			 * Bounding box for the geometry which can be calculated with `computeBoundingBox()`.
			 *
			 * @type {?Box3}
			 * @default null
			 */
			this.boundingBox = null;

			/**
			 * Bounding sphere for the geometry which can be calculated with `computeBoundingSphere()`.
			 *
			 * @type {?Sphere}
			 * @default null
			 */
			this.boundingSphere = null;

			/**
			 * Determines the part of the geometry to render. This should not be set directly,
			 * instead use `setDrawRange()`.
			 *
			 * @type {{start:number,count:number}}
			 */
			this.drawRange = { start: 0, count: Infinity };

			/**
			 * An object that can be used to store custom data about the geometry.
			 * It should not hold references to functions as these will not be cloned.
			 *
			 * @type {Object}
			 */
			this.userData = {};

		}

		/**
		 * Returns the index of this geometry.
		 *
		 * @return {?BufferAttribute} The index. Returns `null` if no index is defined.
		 */
		getIndex() {

			return this.index;

		}

		/**
		 * Sets the given index to this geometry.
		 *
		 * @param {Array<number>|BufferAttribute} index - The index to set.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		setIndex( index ) {

			if ( Array.isArray( index ) ) {

				this.index = new ( arrayNeedsUint32( index ) ? Uint32BufferAttribute : Uint16BufferAttribute )( index, 1 );

			} else {

				this.index = index;

			}

			return this;

		}

		/**
		 * Sets the given indirect attribute to this geometry.
		 *
		 * @param {BufferAttribute} indirect - The attribute holding indirect draw calls.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		setIndirect( indirect ) {

			this.indirect = indirect;

			return this;

		}

		/**
		 * Returns the indirect attribute of this geometry.
		 *
		 * @return {?BufferAttribute} The indirect attribute. Returns `null` if no indirect attribute is defined.
		 */
		getIndirect() {

			return this.indirect;

		}

		/**
		 * Returns the buffer attribute for the given name.
		 *
		 * @param {string} name - The attribute name.
		 * @return {BufferAttribute|InterleavedBufferAttribute|undefined} The buffer attribute.
		 * Returns `undefined` if not attribute has been found.
		 */
		getAttribute( name ) {

			return this.attributes[ name ];

		}

		/**
		 * Sets the given attribute for the given name.
		 *
		 * @param {string} name - The attribute name.
		 * @param {BufferAttribute|InterleavedBufferAttribute} attribute - The attribute to set.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		setAttribute( name, attribute ) {

			this.attributes[ name ] = attribute;

			return this;

		}

		/**
		 * Deletes the attribute for the given name.
		 *
		 * @param {string} name - The attribute name to delete.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		deleteAttribute( name ) {

			delete this.attributes[ name ];

			return this;

		}

		/**
		 * Returns `true` if this geometry has an attribute for the given name.
		 *
		 * @param {string} name - The attribute name.
		 * @return {boolean} Whether this geometry has an attribute for the given name or not.
		 */
		hasAttribute( name ) {

			return this.attributes[ name ] !== undefined;

		}

		/**
		 * Adds a group to this geometry.
		 *
		 * @param {number} start - The first element in this draw call. That is the first
		 * vertex for non-indexed geometry, otherwise the first triangle index.
		 * @param {number} count - Specifies how many vertices (or indices) are part of this group.
		 * @param {number} [materialIndex=0] - The material array index to use.
		 */
		addGroup( start, count, materialIndex = 0 ) {

			this.groups.push( {

				start: start,
				count: count,
				materialIndex: materialIndex

			} );

		}

		/**
		 * Clears all groups.
		 */
		clearGroups() {

			this.groups = [];

		}

		/**
		 * Sets the draw range for this geometry.
		 *
		 * @param {number} start - The first vertex for non-indexed geometry, otherwise the first triangle index.
		 * @param {number} count - For non-indexed BufferGeometry, `count` is the number of vertices to render.
		 * For indexed BufferGeometry, `count` is the number of indices to render.
		 */
		setDrawRange( start, count ) {

			this.drawRange.start = start;
			this.drawRange.count = count;

		}

		/**
		 * Applies the given 4x4 transformation matrix to the geometry.
		 *
		 * @param {Matrix4} matrix - The matrix to apply.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		applyMatrix4( matrix ) {

			const position = this.attributes.position;

			if ( position !== undefined ) {

				position.applyMatrix4( matrix );

				position.needsUpdate = true;

			}

			const normal = this.attributes.normal;

			if ( normal !== undefined ) {

				const normalMatrix = new Matrix3().getNormalMatrix( matrix );

				normal.applyNormalMatrix( normalMatrix );

				normal.needsUpdate = true;

			}

			const tangent = this.attributes.tangent;

			if ( tangent !== undefined ) {

				tangent.transformDirection( matrix );

				tangent.needsUpdate = true;

			}

			if ( this.boundingBox !== null ) {

				this.computeBoundingBox();

			}

			if ( this.boundingSphere !== null ) {

				this.computeBoundingSphere();

			}

			return this;

		}

		/**
		 * Applies the rotation represented by the Quaternion to the geometry.
		 *
		 * @param {Quaternion} q - The Quaternion to apply.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		applyQuaternion( q ) {

			_m1.makeRotationFromQuaternion( q );

			this.applyMatrix4( _m1 );

			return this;

		}

		/**
		 * Rotates the geometry about the X axis. This is typically done as a one time
		 * operation, and not during a loop. Use {@link Object3D#rotation} for typical
		 * real-time mesh rotation.
		 *
		 * @param {number} angle - The angle in radians.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		rotateX( angle ) {

			// rotate geometry around world x-axis

			_m1.makeRotationX( angle );

			this.applyMatrix4( _m1 );

			return this;

		}

		/**
		 * Rotates the geometry about the Y axis. This is typically done as a one time
		 * operation, and not during a loop. Use {@link Object3D#rotation} for typical
		 * real-time mesh rotation.
		 *
		 * @param {number} angle - The angle in radians.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		rotateY( angle ) {

			// rotate geometry around world y-axis

			_m1.makeRotationY( angle );

			this.applyMatrix4( _m1 );

			return this;

		}

		/**
		 * Rotates the geometry about the Z axis. This is typically done as a one time
		 * operation, and not during a loop. Use {@link Object3D#rotation} for typical
		 * real-time mesh rotation.
		 *
		 * @param {number} angle - The angle in radians.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		rotateZ( angle ) {

			// rotate geometry around world z-axis

			_m1.makeRotationZ( angle );

			this.applyMatrix4( _m1 );

			return this;

		}

		/**
		 * Translates the geometry. This is typically done as a one time
		 * operation, and not during a loop. Use {@link Object3D#position} for typical
		 * real-time mesh rotation.
		 *
		 * @param {number} x - The x offset.
		 * @param {number} y - The y offset.
		 * @param {number} z - The z offset.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		translate( x, y, z ) {

			// translate geometry

			_m1.makeTranslation( x, y, z );

			this.applyMatrix4( _m1 );

			return this;

		}

		/**
		 * Scales the geometry. This is typically done as a one time
		 * operation, and not during a loop. Use {@link Object3D#scale} for typical
		 * real-time mesh rotation.
		 *
		 * @param {number} x - The x scale.
		 * @param {number} y - The y scale.
		 * @param {number} z - The z scale.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		scale( x, y, z ) {

			// scale geometry

			_m1.makeScale( x, y, z );

			this.applyMatrix4( _m1 );

			return this;

		}

		/**
		 * Rotates the geometry to face a point in 3D space. This is typically done as a one time
		 * operation, and not during a loop. Use {@link Object3D#lookAt} for typical
		 * real-time mesh rotation.
		 *
		 * @param {Vector3} vector - The target point.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		lookAt( vector ) {

			_obj.lookAt( vector );

			_obj.updateMatrix();

			this.applyMatrix4( _obj.matrix );

			return this;

		}

		/**
		 * Center the geometry based on its bounding box.
		 *
		 * @return {BufferGeometry} A reference to this instance.
		 */
		center() {

			this.computeBoundingBox();

			this.boundingBox.getCenter( _offset ).negate();

			this.translate( _offset.x, _offset.y, _offset.z );

			return this;

		}

		/**
		 * Defines a geometry by creating a `position` attribute based on the given array of points. The array
		 * can hold 2D or 3D vectors. When using two-dimensional data, the `z` coordinate for all vertices is
		 * set to `0`.
		 *
		 * If the method is used with an existing `position` attribute, the vertex data are overwritten with the
		 * data from the array. The length of the array must match the vertex count.
		 *
		 * @param {Array<Vector2>|Array<Vector3>} points - The points.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		setFromPoints( points ) {

			const positionAttribute = this.getAttribute( 'position' );

			if ( positionAttribute === undefined ) {

				const position = [];

				for ( let i = 0, l = points.length; i < l; i ++ ) {

					const point = points[ i ];
					position.push( point.x, point.y, point.z || 0 );

				}

				this.setAttribute( 'position', new Float32BufferAttribute( position, 3 ) );

			} else {

				const l = Math.min( points.length, positionAttribute.count ); // make sure data do not exceed buffer size

				for ( let i = 0; i < l; i ++ ) {

					const point = points[ i ];
					positionAttribute.setXYZ( i, point.x, point.y, point.z || 0 );

				}

				if ( points.length > positionAttribute.count ) {

					console.warn( 'THREE.BufferGeometry: Buffer size too small for points data. Use .dispose() and create a new geometry.' );

				}

				positionAttribute.needsUpdate = true;

			}

			return this;

		}

		/**
		 * Computes the bounding box of the geometry, and updates the `boundingBox` member.
		 * The bounding box is not computed by the engine; it must be computed by your app.
		 * You may need to recompute the bounding box if the geometry vertices are modified.
		 */
		computeBoundingBox() {

			if ( this.boundingBox === null ) {

				this.boundingBox = new Box3();

			}

			const position = this.attributes.position;
			const morphAttributesPosition = this.morphAttributes.position;

			if ( position && position.isGLBufferAttribute ) {

				console.error( 'THREE.BufferGeometry.computeBoundingBox(): GLBufferAttribute requires a manual bounding box.', this );

				this.boundingBox.set(
					new Vector3( - Infinity, - Infinity, - Infinity ),
					new Vector3( + Infinity, + Infinity, + Infinity )
				);

				return;

			}

			if ( position !== undefined ) {

				this.boundingBox.setFromBufferAttribute( position );

				// process morph attributes if present

				if ( morphAttributesPosition ) {

					for ( let i = 0, il = morphAttributesPosition.length; i < il; i ++ ) {

						const morphAttribute = morphAttributesPosition[ i ];
						_box.setFromBufferAttribute( morphAttribute );

						if ( this.morphTargetsRelative ) {

							_vector$1.addVectors( this.boundingBox.min, _box.min );
							this.boundingBox.expandByPoint( _vector$1 );

							_vector$1.addVectors( this.boundingBox.max, _box.max );
							this.boundingBox.expandByPoint( _vector$1 );

						} else {

							this.boundingBox.expandByPoint( _box.min );
							this.boundingBox.expandByPoint( _box.max );

						}

					}

				}

			} else {

				this.boundingBox.makeEmpty();

			}

			if ( isNaN( this.boundingBox.min.x ) || isNaN( this.boundingBox.min.y ) || isNaN( this.boundingBox.min.z ) ) {

				console.error( 'THREE.BufferGeometry.computeBoundingBox(): Computed min/max have NaN values. The "position" attribute is likely to have NaN values.', this );

			}

		}

		/**
		 * Computes the bounding sphere of the geometry, and updates the `boundingSphere` member.
		 * The engine automatically computes the bounding sphere when it is needed, e.g., for ray casting or view frustum culling.
		 * You may need to recompute the bounding sphere if the geometry vertices are modified.
		 */
		computeBoundingSphere() {

			if ( this.boundingSphere === null ) {

				this.boundingSphere = new Sphere();

			}

			const position = this.attributes.position;
			const morphAttributesPosition = this.morphAttributes.position;

			if ( position && position.isGLBufferAttribute ) {

				console.error( 'THREE.BufferGeometry.computeBoundingSphere(): GLBufferAttribute requires a manual bounding sphere.', this );

				this.boundingSphere.set( new Vector3(), Infinity );

				return;

			}

			if ( position ) {

				// first, find the center of the bounding sphere

				const center = this.boundingSphere.center;

				_box.setFromBufferAttribute( position );

				// process morph attributes if present

				if ( morphAttributesPosition ) {

					for ( let i = 0, il = morphAttributesPosition.length; i < il; i ++ ) {

						const morphAttribute = morphAttributesPosition[ i ];
						_boxMorphTargets.setFromBufferAttribute( morphAttribute );

						if ( this.morphTargetsRelative ) {

							_vector$1.addVectors( _box.min, _boxMorphTargets.min );
							_box.expandByPoint( _vector$1 );

							_vector$1.addVectors( _box.max, _boxMorphTargets.max );
							_box.expandByPoint( _vector$1 );

						} else {

							_box.expandByPoint( _boxMorphTargets.min );
							_box.expandByPoint( _boxMorphTargets.max );

						}

					}

				}

				_box.getCenter( center );

				// second, try to find a boundingSphere with a radius smaller than the
				// boundingSphere of the boundingBox: sqrt(3) smaller in the best case

				let maxRadiusSq = 0;

				for ( let i = 0, il = position.count; i < il; i ++ ) {

					_vector$1.fromBufferAttribute( position, i );

					maxRadiusSq = Math.max( maxRadiusSq, center.distanceToSquared( _vector$1 ) );

				}

				// process morph attributes if present

				if ( morphAttributesPosition ) {

					for ( let i = 0, il = morphAttributesPosition.length; i < il; i ++ ) {

						const morphAttribute = morphAttributesPosition[ i ];
						const morphTargetsRelative = this.morphTargetsRelative;

						for ( let j = 0, jl = morphAttribute.count; j < jl; j ++ ) {

							_vector$1.fromBufferAttribute( morphAttribute, j );

							if ( morphTargetsRelative ) {

								_offset.fromBufferAttribute( position, j );
								_vector$1.add( _offset );

							}

							maxRadiusSq = Math.max( maxRadiusSq, center.distanceToSquared( _vector$1 ) );

						}

					}

				}

				this.boundingSphere.radius = Math.sqrt( maxRadiusSq );

				if ( isNaN( this.boundingSphere.radius ) ) {

					console.error( 'THREE.BufferGeometry.computeBoundingSphere(): Computed radius is NaN. The "position" attribute is likely to have NaN values.', this );

				}

			}

		}

		/**
		 * Calculates and adds a tangent attribute to this geometry.
		 *
		 * The computation is only supported for indexed geometries and if position, normal, and uv attributes
		 * are defined. When using a tangent space normal map, prefer the MikkTSpace algorithm provided by
		 * {@link BufferGeometryUtils#computeMikkTSpaceTangents} instead.
		 */
		computeTangents() {

			const index = this.index;
			const attributes = this.attributes;

			// based on http://www.terathon.com/code/tangent.html
			// (per vertex tangents)

			if ( index === null ||
				 attributes.position === undefined ||
				 attributes.normal === undefined ||
				 attributes.uv === undefined ) {

				console.error( 'THREE.BufferGeometry: .computeTangents() failed. Missing required attributes (index, position, normal or uv)' );
				return;

			}

			const positionAttribute = attributes.position;
			const normalAttribute = attributes.normal;
			const uvAttribute = attributes.uv;

			if ( this.hasAttribute( 'tangent' ) === false ) {

				this.setAttribute( 'tangent', new BufferAttribute( new Float32Array( 4 * positionAttribute.count ), 4 ) );

			}

			const tangentAttribute = this.getAttribute( 'tangent' );

			const tan1 = [], tan2 = [];

			for ( let i = 0; i < positionAttribute.count; i ++ ) {

				tan1[ i ] = new Vector3();
				tan2[ i ] = new Vector3();

			}

			const vA = new Vector3(),
				vB = new Vector3(),
				vC = new Vector3(),

				uvA = new Vector2(),
				uvB = new Vector2(),
				uvC = new Vector2(),

				sdir = new Vector3(),
				tdir = new Vector3();

			function handleTriangle( a, b, c ) {

				vA.fromBufferAttribute( positionAttribute, a );
				vB.fromBufferAttribute( positionAttribute, b );
				vC.fromBufferAttribute( positionAttribute, c );

				uvA.fromBufferAttribute( uvAttribute, a );
				uvB.fromBufferAttribute( uvAttribute, b );
				uvC.fromBufferAttribute( uvAttribute, c );

				vB.sub( vA );
				vC.sub( vA );

				uvB.sub( uvA );
				uvC.sub( uvA );

				const r = 1.0 / ( uvB.x * uvC.y - uvC.x * uvB.y );

				// silently ignore degenerate uv triangles having coincident or colinear vertices

				if ( ! isFinite( r ) ) return;

				sdir.copy( vB ).multiplyScalar( uvC.y ).addScaledVector( vC, - uvB.y ).multiplyScalar( r );
				tdir.copy( vC ).multiplyScalar( uvB.x ).addScaledVector( vB, - uvC.x ).multiplyScalar( r );

				tan1[ a ].add( sdir );
				tan1[ b ].add( sdir );
				tan1[ c ].add( sdir );

				tan2[ a ].add( tdir );
				tan2[ b ].add( tdir );
				tan2[ c ].add( tdir );

			}

			let groups = this.groups;

			if ( groups.length === 0 ) {

				groups = [ {
					start: 0,
					count: index.count
				} ];

			}

			for ( let i = 0, il = groups.length; i < il; ++ i ) {

				const group = groups[ i ];

				const start = group.start;
				const count = group.count;

				for ( let j = start, jl = start + count; j < jl; j += 3 ) {

					handleTriangle(
						index.getX( j + 0 ),
						index.getX( j + 1 ),
						index.getX( j + 2 )
					);

				}

			}

			const tmp = new Vector3(), tmp2 = new Vector3();
			const n = new Vector3(), n2 = new Vector3();

			function handleVertex( v ) {

				n.fromBufferAttribute( normalAttribute, v );
				n2.copy( n );

				const t = tan1[ v ];

				// Gram-Schmidt orthogonalize

				tmp.copy( t );
				tmp.sub( n.multiplyScalar( n.dot( t ) ) ).normalize();

				// Calculate handedness

				tmp2.crossVectors( n2, t );
				const test = tmp2.dot( tan2[ v ] );
				const w = ( test < 0.0 ) ? - 1.0 : 1.0;

				tangentAttribute.setXYZW( v, tmp.x, tmp.y, tmp.z, w );

			}

			for ( let i = 0, il = groups.length; i < il; ++ i ) {

				const group = groups[ i ];

				const start = group.start;
				const count = group.count;

				for ( let j = start, jl = start + count; j < jl; j += 3 ) {

					handleVertex( index.getX( j + 0 ) );
					handleVertex( index.getX( j + 1 ) );
					handleVertex( index.getX( j + 2 ) );

				}

			}

		}

		/**
		 * Computes vertex normals for the given vertex data. For indexed geometries, the method sets
		 * each vertex normal to be the average of the face normals of the faces that share that vertex.
		 * For non-indexed geometries, vertices are not shared, and the method sets each vertex normal
		 * to be the same as the face normal.
		 */
		computeVertexNormals() {

			const index = this.index;
			const positionAttribute = this.getAttribute( 'position' );

			if ( positionAttribute !== undefined ) {

				let normalAttribute = this.getAttribute( 'normal' );

				if ( normalAttribute === undefined ) {

					normalAttribute = new BufferAttribute( new Float32Array( positionAttribute.count * 3 ), 3 );
					this.setAttribute( 'normal', normalAttribute );

				} else {

					// reset existing normals to zero

					for ( let i = 0, il = normalAttribute.count; i < il; i ++ ) {

						normalAttribute.setXYZ( i, 0, 0, 0 );

					}

				}

				const pA = new Vector3(), pB = new Vector3(), pC = new Vector3();
				const nA = new Vector3(), nB = new Vector3(), nC = new Vector3();
				const cb = new Vector3(), ab = new Vector3();

				// indexed elements

				if ( index ) {

					for ( let i = 0, il = index.count; i < il; i += 3 ) {

						const vA = index.getX( i + 0 );
						const vB = index.getX( i + 1 );
						const vC = index.getX( i + 2 );

						pA.fromBufferAttribute( positionAttribute, vA );
						pB.fromBufferAttribute( positionAttribute, vB );
						pC.fromBufferAttribute( positionAttribute, vC );

						cb.subVectors( pC, pB );
						ab.subVectors( pA, pB );
						cb.cross( ab );

						nA.fromBufferAttribute( normalAttribute, vA );
						nB.fromBufferAttribute( normalAttribute, vB );
						nC.fromBufferAttribute( normalAttribute, vC );

						nA.add( cb );
						nB.add( cb );
						nC.add( cb );

						normalAttribute.setXYZ( vA, nA.x, nA.y, nA.z );
						normalAttribute.setXYZ( vB, nB.x, nB.y, nB.z );
						normalAttribute.setXYZ( vC, nC.x, nC.y, nC.z );

					}

				} else {

					// non-indexed elements (unconnected triangle soup)

					for ( let i = 0, il = positionAttribute.count; i < il; i += 3 ) {

						pA.fromBufferAttribute( positionAttribute, i + 0 );
						pB.fromBufferAttribute( positionAttribute, i + 1 );
						pC.fromBufferAttribute( positionAttribute, i + 2 );

						cb.subVectors( pC, pB );
						ab.subVectors( pA, pB );
						cb.cross( ab );

						normalAttribute.setXYZ( i + 0, cb.x, cb.y, cb.z );
						normalAttribute.setXYZ( i + 1, cb.x, cb.y, cb.z );
						normalAttribute.setXYZ( i + 2, cb.x, cb.y, cb.z );

					}

				}

				this.normalizeNormals();

				normalAttribute.needsUpdate = true;

			}

		}

		/**
		 * Ensures every normal vector in a geometry will have a magnitude of `1`. This will
		 * correct lighting on the geometry surfaces.
		 */
		normalizeNormals() {

			const normals = this.attributes.normal;

			for ( let i = 0, il = normals.count; i < il; i ++ ) {

				_vector$1.fromBufferAttribute( normals, i );

				_vector$1.normalize();

				normals.setXYZ( i, _vector$1.x, _vector$1.y, _vector$1.z );

			}

		}

		/**
		 * Return a new non-index version of this indexed geometry. If the geometry
		 * is already non-indexed, the method is a NOOP.
		 *
		 * @return {BufferGeometry} The non-indexed version of this indexed geometry.
		 */
		toNonIndexed() {

			function convertBufferAttribute( attribute, indices ) {

				const array = attribute.array;
				const itemSize = attribute.itemSize;
				const normalized = attribute.normalized;

				const array2 = new array.constructor( indices.length * itemSize );

				let index = 0, index2 = 0;

				for ( let i = 0, l = indices.length; i < l; i ++ ) {

					if ( attribute.isInterleavedBufferAttribute ) {

						index = indices[ i ] * attribute.data.stride + attribute.offset;

					} else {

						index = indices[ i ] * itemSize;

					}

					for ( let j = 0; j < itemSize; j ++ ) {

						array2[ index2 ++ ] = array[ index ++ ];

					}

				}

				return new BufferAttribute( array2, itemSize, normalized );

			}

			//

			if ( this.index === null ) {

				console.warn( 'THREE.BufferGeometry.toNonIndexed(): BufferGeometry is already non-indexed.' );
				return this;

			}

			const geometry2 = new BufferGeometry();

			const indices = this.index.array;
			const attributes = this.attributes;

			// attributes

			for ( const name in attributes ) {

				const attribute = attributes[ name ];

				const newAttribute = convertBufferAttribute( attribute, indices );

				geometry2.setAttribute( name, newAttribute );

			}

			// morph attributes

			const morphAttributes = this.morphAttributes;

			for ( const name in morphAttributes ) {

				const morphArray = [];
				const morphAttribute = morphAttributes[ name ]; // morphAttribute: array of Float32BufferAttributes

				for ( let i = 0, il = morphAttribute.length; i < il; i ++ ) {

					const attribute = morphAttribute[ i ];

					const newAttribute = convertBufferAttribute( attribute, indices );

					morphArray.push( newAttribute );

				}

				geometry2.morphAttributes[ name ] = morphArray;

			}

			geometry2.morphTargetsRelative = this.morphTargetsRelative;

			// groups

			const groups = this.groups;

			for ( let i = 0, l = groups.length; i < l; i ++ ) {

				const group = groups[ i ];
				geometry2.addGroup( group.start, group.count, group.materialIndex );

			}

			return geometry2;

		}

		/**
		 * Serializes the geometry into JSON.
		 *
		 * @return {Object} A JSON object representing the serialized geometry.
		 */
		toJSON() {

			const data = {
				metadata: {
					version: 4.7,
					type: 'BufferGeometry',
					generator: 'BufferGeometry.toJSON'
				}
			};

			// standard BufferGeometry serialization

			data.uuid = this.uuid;
			data.type = this.type;
			if ( this.name !== '' ) data.name = this.name;
			if ( Object.keys( this.userData ).length > 0 ) data.userData = this.userData;

			if ( this.parameters !== undefined ) {

				const parameters = this.parameters;

				for ( const key in parameters ) {

					if ( parameters[ key ] !== undefined ) data[ key ] = parameters[ key ];

				}

				return data;

			}

			// for simplicity the code assumes attributes are not shared across geometries, see #15811

			data.data = { attributes: {} };

			const index = this.index;

			if ( index !== null ) {

				data.data.index = {
					type: index.array.constructor.name,
					array: Array.prototype.slice.call( index.array )
				};

			}

			const attributes = this.attributes;

			for ( const key in attributes ) {

				const attribute = attributes[ key ];

				data.data.attributes[ key ] = attribute.toJSON( data.data );

			}

			const morphAttributes = {};
			let hasMorphAttributes = false;

			for ( const key in this.morphAttributes ) {

				const attributeArray = this.morphAttributes[ key ];

				const array = [];

				for ( let i = 0, il = attributeArray.length; i < il; i ++ ) {

					const attribute = attributeArray[ i ];

					array.push( attribute.toJSON( data.data ) );

				}

				if ( array.length > 0 ) {

					morphAttributes[ key ] = array;

					hasMorphAttributes = true;

				}

			}

			if ( hasMorphAttributes ) {

				data.data.morphAttributes = morphAttributes;
				data.data.morphTargetsRelative = this.morphTargetsRelative;

			}

			const groups = this.groups;

			if ( groups.length > 0 ) {

				data.data.groups = JSON.parse( JSON.stringify( groups ) );

			}

			const boundingSphere = this.boundingSphere;

			if ( boundingSphere !== null ) {

				data.data.boundingSphere = boundingSphere.toJSON();

			}

			return data;

		}

		/**
		 * Returns a new geometry with copied values from this instance.
		 *
		 * @return {BufferGeometry} A clone of this instance.
		 */
		clone() {

			return new this.constructor().copy( this );

		}

		/**
		 * Copies the values of the given geometry to this instance.
		 *
		 * @param {BufferGeometry} source - The geometry to copy.
		 * @return {BufferGeometry} A reference to this instance.
		 */
		copy( source ) {

			// reset

			this.index = null;
			this.attributes = {};
			this.morphAttributes = {};
			this.groups = [];
			this.boundingBox = null;
			this.boundingSphere = null;

			// used for storing cloned, shared data

			const data = {};

			// name

			this.name = source.name;

			// index

			const index = source.index;

			if ( index !== null ) {

				this.setIndex( index.clone() );

			}

			// attributes

			const attributes = source.attributes;

			for ( const name in attributes ) {

				const attribute = attributes[ name ];
				this.setAttribute( name, attribute.clone( data ) );

			}

			// morph attributes

			const morphAttributes = source.morphAttributes;

			for ( const name in morphAttributes ) {

				const array = [];
				const morphAttribute = morphAttributes[ name ]; // morphAttribute: array of Float32BufferAttributes

				for ( let i = 0, l = morphAttribute.length; i < l; i ++ ) {

					array.push( morphAttribute[ i ].clone( data ) );

				}

				this.morphAttributes[ name ] = array;

			}

			this.morphTargetsRelative = source.morphTargetsRelative;

			// groups

			const groups = source.groups;

			for ( let i = 0, l = groups.length; i < l; i ++ ) {

				const group = groups[ i ];
				this.addGroup( group.start, group.count, group.materialIndex );

			}

			// bounding box

			const boundingBox = source.boundingBox;

			if ( boundingBox !== null ) {

				this.boundingBox = boundingBox.clone();

			}

			// bounding sphere

			const boundingSphere = source.boundingSphere;

			if ( boundingSphere !== null ) {

				this.boundingSphere = boundingSphere.clone();

			}

			// draw range

			this.drawRange.start = source.drawRange.start;
			this.drawRange.count = source.drawRange.count;

			// user data

			this.userData = source.userData;

			return this;

		}

		/**
		 * Frees the GPU-related resources allocated by this instance. Call this
		 * method whenever this instance is no longer used in your app.
		 *
		 * @fires BufferGeometry#dispose
		 */
		dispose() {

			this.dispatchEvent( { type: 'dispose' } );

		}

	}

	/**
	 * A geometry class for a rectangular cuboid with a given width, height, and depth.
	 * On creation, the cuboid is centred on the origin, with each edge parallel to one
	 * of the axes.
	 *
	 * ```js
	 * const geometry = new THREE.BoxGeometry( 1, 1, 1 );
	 * const material = new THREE.MeshBasicMaterial( { color: 0x00ff00 } );
	 * const cube = new THREE.Mesh( geometry, material );
	 * scene.add( cube );
	 * ```
	 *
	 * @augments BufferGeometry
	 */
	class BoxGeometry extends BufferGeometry {

		/**
		 * Constructs a new box geometry.
		 *
		 * @param {number} [width=1] - The width. That is, the length of the edges parallel to the X axis.
		 * @param {number} [height=1] - The height. That is, the length of the edges parallel to the Y axis.
		 * @param {number} [depth=1] - The depth. That is, the length of the edges parallel to the Z axis.
		 * @param {number} [widthSegments=1] - Number of segmented rectangular faces along the width of the sides.
		 * @param {number} [heightSegments=1] - Number of segmented rectangular faces along the height of the sides.
		 * @param {number} [depthSegments=1] - Number of segmented rectangular faces along the depth of the sides.
		 */
		constructor( width = 1, height = 1, depth = 1, widthSegments = 1, heightSegments = 1, depthSegments = 1 ) {

			super();

			this.type = 'BoxGeometry';

			/**
			 * Holds the constructor parameters that have been
			 * used to generate the geometry. Any modification
			 * after instantiation does not change the geometry.
			 *
			 * @type {Object}
			 */
			this.parameters = {
				width: width,
				height: height,
				depth: depth,
				widthSegments: widthSegments,
				heightSegments: heightSegments,
				depthSegments: depthSegments
			};

			const scope = this;

			// segments

			widthSegments = Math.floor( widthSegments );
			heightSegments = Math.floor( heightSegments );
			depthSegments = Math.floor( depthSegments );

			// buffers

			const indices = [];
			const vertices = [];
			const normals = [];
			const uvs = [];

			// helper variables

			let numberOfVertices = 0;
			let groupStart = 0;

			// build each side of the box geometry

			buildPlane( 'z', 'y', 'x', - 1, - 1, depth, height, width, depthSegments, heightSegments, 0 ); // px
			buildPlane( 'z', 'y', 'x', 1, - 1, depth, height, - width, depthSegments, heightSegments, 1 ); // nx
			buildPlane( 'x', 'z', 'y', 1, 1, width, depth, height, widthSegments, depthSegments, 2 ); // py
			buildPlane( 'x', 'z', 'y', 1, - 1, width, depth, - height, widthSegments, depthSegments, 3 ); // ny
			buildPlane( 'x', 'y', 'z', 1, - 1, width, height, depth, widthSegments, heightSegments, 4 ); // pz
			buildPlane( 'x', 'y', 'z', - 1, - 1, width, height, - depth, widthSegments, heightSegments, 5 ); // nz

			// build geometry

			this.setIndex( indices );
			this.setAttribute( 'position', new Float32BufferAttribute( vertices, 3 ) );
			this.setAttribute( 'normal', new Float32BufferAttribute( normals, 3 ) );
			this.setAttribute( 'uv', new Float32BufferAttribute( uvs, 2 ) );

			function buildPlane( u, v, w, udir, vdir, width, height, depth, gridX, gridY, materialIndex ) {

				const segmentWidth = width / gridX;
				const segmentHeight = height / gridY;

				const widthHalf = width / 2;
				const heightHalf = height / 2;
				const depthHalf = depth / 2;

				const gridX1 = gridX + 1;
				const gridY1 = gridY + 1;

				let vertexCounter = 0;
				let groupCount = 0;

				const vector = new Vector3();

				// generate vertices, normals and uvs

				for ( let iy = 0; iy < gridY1; iy ++ ) {

					const y = iy * segmentHeight - heightHalf;

					for ( let ix = 0; ix < gridX1; ix ++ ) {

						const x = ix * segmentWidth - widthHalf;

						// set values to correct vector component

						vector[ u ] = x * udir;
						vector[ v ] = y * vdir;
						vector[ w ] = depthHalf;

						// now apply vector to vertex buffer

						vertices.push( vector.x, vector.y, vector.z );

						// set values to correct vector component

						vector[ u ] = 0;
						vector[ v ] = 0;
						vector[ w ] = depth > 0 ? 1 : - 1;

						// now apply vector to normal buffer

						normals.push( vector.x, vector.y, vector.z );

						// uvs

						uvs.push( ix / gridX );
						uvs.push( 1 - ( iy / gridY ) );

						// counters

						vertexCounter += 1;

					}

				}

				// indices

				// 1. you need three indices to draw a single face
				// 2. a single segment consists of two faces
				// 3. so we need to generate six (2*3) indices per segment

				for ( let iy = 0; iy < gridY; iy ++ ) {

					for ( let ix = 0; ix < gridX; ix ++ ) {

						const a = numberOfVertices + ix + gridX1 * iy;
						const b = numberOfVertices + ix + gridX1 * ( iy + 1 );
						const c = numberOfVertices + ( ix + 1 ) + gridX1 * ( iy + 1 );
						const d = numberOfVertices + ( ix + 1 ) + gridX1 * iy;

						// faces

						indices.push( a, b, d );
						indices.push( b, c, d );

						// increase counter

						groupCount += 6;

					}

				}

				// add a group to the geometry. this will ensure multi material support

				scope.addGroup( groupStart, groupCount, materialIndex );

				// calculate new start value for groups

				groupStart += groupCount;

				// update total number of vertices

				numberOfVertices += vertexCounter;

			}

		}

		copy( source ) {

			super.copy( source );

			this.parameters = Object.assign( {}, source.parameters );

			return this;

		}

		/**
		 * Factory method for creating an instance of this class from the given
		 * JSON object.
		 *
		 * @param {Object} data - A JSON object representing the serialized geometry.
		 * @return {BoxGeometry} A new instance.
		 */
		static fromJSON( data ) {

			return new BoxGeometry( data.width, data.height, data.depth, data.widthSegments, data.heightSegments, data.depthSegments );

		}

	}

	/**
	 * A class for generating a sphere geometry.
	 *
	 * ```js
	 * const geometry = new THREE.SphereGeometry( 15, 32, 16 );
	 * const material = new THREE.MeshBasicMaterial( { color: 0xffff00 } );
	 * const sphere = new THREE.Mesh( geometry, material );
	 * scene.add( sphere );
	 * ```
	 *
	 * @augments BufferGeometry
	 */
	class SphereGeometry extends BufferGeometry {

		/**
		 * Constructs a new sphere geometry.
		 *
		 * @param {number} [radius=1] - The sphere radius.
		 * @param {number} [widthSegments=32] - The number of horizontal segments. Minimum value is `3`.
		 * @param {number} [heightSegments=16] - The number of vertical segments. Minimum value is `2`.
		 * @param {number} [phiStart=0] - The horizontal starting angle in radians.
		 * @param {number} [phiLength=Math.PI*2] - The horizontal sweep angle size.
		 * @param {number} [thetaStart=0] - The vertical starting angle in radians.
		 * @param {number} [thetaLength=Math.PI] - The vertical sweep angle size.
		 */
		constructor( radius = 1, widthSegments = 32, heightSegments = 16, phiStart = 0, phiLength = Math.PI * 2, thetaStart = 0, thetaLength = Math.PI ) {

			super();

			this.type = 'SphereGeometry';

			/**
			 * Holds the constructor parameters that have been
			 * used to generate the geometry. Any modification
			 * after instantiation does not change the geometry.
			 *
			 * @type {Object}
			 */
			this.parameters = {
				radius: radius,
				widthSegments: widthSegments,
				heightSegments: heightSegments,
				phiStart: phiStart,
				phiLength: phiLength,
				thetaStart: thetaStart,
				thetaLength: thetaLength
			};

			widthSegments = Math.max( 3, Math.floor( widthSegments ) );
			heightSegments = Math.max( 2, Math.floor( heightSegments ) );

			const thetaEnd = Math.min( thetaStart + thetaLength, Math.PI );

			let index = 0;
			const grid = [];

			const vertex = new Vector3();
			const normal = new Vector3();

			// buffers

			const indices = [];
			const vertices = [];
			const normals = [];
			const uvs = [];

			// generate vertices, normals and uvs

			for ( let iy = 0; iy <= heightSegments; iy ++ ) {

				const verticesRow = [];

				const v = iy / heightSegments;

				// special case for the poles

				let uOffset = 0;

				if ( iy === 0 && thetaStart === 0 ) {

					uOffset = 0.5 / widthSegments;

				} else if ( iy === heightSegments && thetaEnd === Math.PI ) {

					uOffset = - 0.5 / widthSegments;

				}

				for ( let ix = 0; ix <= widthSegments; ix ++ ) {

					const u = ix / widthSegments;

					// vertex

					vertex.x = - radius * Math.cos( phiStart + u * phiLength ) * Math.sin( thetaStart + v * thetaLength );
					vertex.y = radius * Math.cos( thetaStart + v * thetaLength );
					vertex.z = radius * Math.sin( phiStart + u * phiLength ) * Math.sin( thetaStart + v * thetaLength );

					vertices.push( vertex.x, vertex.y, vertex.z );

					// normal

					normal.copy( vertex ).normalize();
					normals.push( normal.x, normal.y, normal.z );

					// uv

					uvs.push( u + uOffset, 1 - v );

					verticesRow.push( index ++ );

				}

				grid.push( verticesRow );

			}

			// indices

			for ( let iy = 0; iy < heightSegments; iy ++ ) {

				for ( let ix = 0; ix < widthSegments; ix ++ ) {

					const a = grid[ iy ][ ix + 1 ];
					const b = grid[ iy ][ ix ];
					const c = grid[ iy + 1 ][ ix ];
					const d = grid[ iy + 1 ][ ix + 1 ];

					if ( iy !== 0 || thetaStart > 0 ) indices.push( a, b, d );
					if ( iy !== heightSegments - 1 || thetaEnd < Math.PI ) indices.push( b, c, d );

				}

			}

			// build geometry

			this.setIndex( indices );
			this.setAttribute( 'position', new Float32BufferAttribute( vertices, 3 ) );
			this.setAttribute( 'normal', new Float32BufferAttribute( normals, 3 ) );
			this.setAttribute( 'uv', new Float32BufferAttribute( uvs, 2 ) );

		}

		copy( source ) {

			super.copy( source );

			this.parameters = Object.assign( {}, source.parameters );

			return this;

		}

		/**
		 * Factory method for creating an instance of this class from the given
		 * JSON object.
		 *
		 * @param {Object} data - A JSON object representing the serialized geometry.
		 * @return {SphereGeometry} A new instance.
		 */
		static fromJSON( data ) {

			return new SphereGeometry( data.radius, data.widthSegments, data.heightSegments, data.phiStart, data.phiLength, data.thetaStart, data.thetaLength );

		}

	}

	let _materialId = 0;

	/**
	 * Abstract base class for materials.
	 *
	 * Materials define the appearance of renderable 3D objects.
	 *
	 * @abstract
	 * @augments EventDispatcher
	 */
	class Material extends EventDispatcher {

		/**
		 * Constructs a new material.
		 */
		constructor() {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isMaterial = true;

			/**
			 * The ID of the material.
			 *
			 * @name Material#id
			 * @type {number}
			 * @readonly
			 */
			Object.defineProperty( this, 'id', { value: _materialId ++ } );

			/**
			 * The UUID of the material.
			 *
			 * @type {string}
			 * @readonly
			 */
			this.uuid = generateUUID();

			/**
			 * The name of the material.
			 *
			 * @type {string}
			 */
			this.name = '';

			/**
			 * The type property is used for detecting the object type
			 * in context of serialization/deserialization.
			 *
			 * @type {string}
			 * @readonly
			 */
			this.type = 'Material';

			/**
			 * Defines the blending type of the material.
			 *
			 * It must be set to `CustomBlending` if custom blending properties like
			 * {@link Material#blendSrc}, {@link Material#blendDst} or {@link Material#blendEquation}
			 * should have any effect.
			 *
			 * @type {(NoBlending|NormalBlending|AdditiveBlending|SubtractiveBlending|MultiplyBlending|CustomBlending)}
			 * @default NormalBlending
			 */
			this.blending = NormalBlending;

			/**
			 * Defines which side of faces will be rendered - front, back or both.
			 *
			 * @type {(FrontSide|BackSide|DoubleSide)}
			 * @default FrontSide
			 */
			this.side = FrontSide;

			/**
			 * If set to `true`, vertex colors should be used.
			 *
			 * The engine supports RGB and RGBA vertex colors depending on whether a three (RGB) or
			 * four (RGBA) component color buffer attribute is used.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.vertexColors = false;

			/**
			 * Defines how transparent the material is.
			 * A value of `0.0` indicates fully transparent, `1.0` is fully opaque.
			 *
			 * If the {@link Material#transparent} is not set to `true`,
			 * the material will remain fully opaque and this value will only affect its color.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.opacity = 1;

			/**
			 * Defines whether this material is transparent. This has an effect on
			 * rendering as transparent objects need special treatment and are rendered
			 * after non-transparent objects.
			 *
			 * When set to true, the extent to which the material is transparent is
			 * controlled by {@link Material#opacity}.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.transparent = false;

			/**
			 * Enables alpha hashed transparency, an alternative to {@link Material#transparent} or
			 * {@link Material#alphaTest}. The material will not be rendered if opacity is lower than
			 * a random threshold. Randomization introduces some grain or noise, but approximates alpha
			 * blending without the associated problems of sorting. Using TAA can reduce the resulting noise.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.alphaHash = false;

			/**
			 * Defines the blending source factor.
			 *
			 * @type {(ZeroFactor|OneFactor|SrcColorFactor|OneMinusSrcColorFactor|SrcAlphaFactor|OneMinusSrcAlphaFactor|DstAlphaFactor|OneMinusDstAlphaFactor|DstColorFactor|OneMinusDstColorFactor|SrcAlphaSaturateFactor|ConstantColorFactor|OneMinusConstantColorFactor|ConstantAlphaFactor|OneMinusConstantAlphaFactor)}
			 * @default SrcAlphaFactor
			 */
			this.blendSrc = SrcAlphaFactor;

			/**
			 * Defines the blending destination factor.
			 *
			 * @type {(ZeroFactor|OneFactor|SrcColorFactor|OneMinusSrcColorFactor|SrcAlphaFactor|OneMinusSrcAlphaFactor|DstAlphaFactor|OneMinusDstAlphaFactor|DstColorFactor|OneMinusDstColorFactor|SrcAlphaSaturateFactor|ConstantColorFactor|OneMinusConstantColorFactor|ConstantAlphaFactor|OneMinusConstantAlphaFactor)}
			 * @default OneMinusSrcAlphaFactor
			 */
			this.blendDst = OneMinusSrcAlphaFactor;

			/**
			 * Defines the blending equation.
			 *
			 * @type {(AddEquation|SubtractEquation|ReverseSubtractEquation|MinEquation|MaxEquation)}
			 * @default AddEquation
			 */
			this.blendEquation = AddEquation;

			/**
			 * Defines the blending source alpha factor.
			 *
			 * @type {?(ZeroFactor|OneFactor|SrcColorFactor|OneMinusSrcColorFactor|SrcAlphaFactor|OneMinusSrcAlphaFactor|DstAlphaFactor|OneMinusDstAlphaFactor|DstColorFactor|OneMinusDstColorFactor|SrcAlphaSaturateFactor|ConstantColorFactor|OneMinusConstantColorFactor|ConstantAlphaFactor|OneMinusConstantAlphaFactor)}
			 * @default null
			 */
			this.blendSrcAlpha = null;

			/**
			 * Defines the blending destination alpha factor.
			 *
			 * @type {?(ZeroFactor|OneFactor|SrcColorFactor|OneMinusSrcColorFactor|SrcAlphaFactor|OneMinusSrcAlphaFactor|DstAlphaFactor|OneMinusDstAlphaFactor|DstColorFactor|OneMinusDstColorFactor|SrcAlphaSaturateFactor|ConstantColorFactor|OneMinusConstantColorFactor|ConstantAlphaFactor|OneMinusConstantAlphaFactor)}
			 * @default null
			 */
			this.blendDstAlpha = null;

			/**
			 * Defines the blending equation of the alpha channel.
			 *
			 * @type {?(AddEquation|SubtractEquation|ReverseSubtractEquation|MinEquation|MaxEquation)}
			 * @default null
			 */
			this.blendEquationAlpha = null;

			/**
			 * Represents the RGB values of the constant blend color.
			 *
			 * This property has only an effect when using custom blending with `ConstantColor` or `OneMinusConstantColor`.
			 *
			 * @type {Color}
			 * @default (0,0,0)
			 */
			this.blendColor = new Color( 0, 0, 0 );

			/**
			 * Represents the alpha value of the constant blend color.
			 *
			 * This property has only an effect when using custom blending with `ConstantAlpha` or `OneMinusConstantAlpha`.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.blendAlpha = 0;

			/**
			 * Defines the depth function.
			 *
			 * @type {(NeverDepth|AlwaysDepth|LessDepth|LessEqualDepth|EqualDepth|GreaterEqualDepth|GreaterDepth|NotEqualDepth)}
			 * @default LessEqualDepth
			 */
			this.depthFunc = LessEqualDepth;

			/**
			 * Whether to have depth test enabled when rendering this material.
			 * When the depth test is disabled, the depth write will also be implicitly disabled.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.depthTest = true;

			/**
			 * Whether rendering this material has any effect on the depth buffer.
			 *
			 * When drawing 2D overlays it can be useful to disable the depth writing in
			 * order to layer several things together without creating z-index artifacts.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.depthWrite = true;

			/**
			 * The bit mask to use when writing to the stencil buffer.
			 *
			 * @type {number}
			 * @default 0xff
			 */
			this.stencilWriteMask = 0xff;

			/**
			 * The stencil comparison function to use.
			 *
			 * @type {NeverStencilFunc|LessStencilFunc|EqualStencilFunc|LessEqualStencilFunc|GreaterStencilFunc|NotEqualStencilFunc|GreaterEqualStencilFunc|AlwaysStencilFunc}
			 * @default AlwaysStencilFunc
			 */
			this.stencilFunc = AlwaysStencilFunc;

			/**
			 * The value to use when performing stencil comparisons or stencil operations.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.stencilRef = 0;

			/**
			 * The bit mask to use when comparing against the stencil buffer.
			 *
			 * @type {number}
			 * @default 0xff
			 */
			this.stencilFuncMask = 0xff;

			/**
			 * Which stencil operation to perform when the comparison function returns `false`.
			 *
			 * @type {ZeroStencilOp|KeepStencilOp|ReplaceStencilOp|IncrementStencilOp|DecrementStencilOp|IncrementWrapStencilOp|DecrementWrapStencilOp|InvertStencilOp}
			 * @default KeepStencilOp
			 */
			this.stencilFail = KeepStencilOp;

			/**
			 * Which stencil operation to perform when the comparison function returns
			 * `true` but the depth test fails.
			 *
			 * @type {ZeroStencilOp|KeepStencilOp|ReplaceStencilOp|IncrementStencilOp|DecrementStencilOp|IncrementWrapStencilOp|DecrementWrapStencilOp|InvertStencilOp}
			 * @default KeepStencilOp
			 */
			this.stencilZFail = KeepStencilOp;

			/**
			 * Which stencil operation to perform when the comparison function returns
			 * `true` and the depth test passes.
			 *
			 * @type {ZeroStencilOp|KeepStencilOp|ReplaceStencilOp|IncrementStencilOp|DecrementStencilOp|IncrementWrapStencilOp|DecrementWrapStencilOp|InvertStencilOp}
			 * @default KeepStencilOp
			 */
			this.stencilZPass = KeepStencilOp;

			/**
			 * Whether stencil operations are performed against the stencil buffer. In
			 * order to perform writes or comparisons against the stencil buffer this
			 * value must be `true`.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.stencilWrite = false;

			/**
			 * User-defined clipping planes specified as THREE.Plane objects in world
			 * space. These planes apply to the objects this material is attached to.
			 * Points in space whose signed distance to the plane is negative are clipped
			 * (not rendered). This requires {@link WebGLRenderer#localClippingEnabled} to
			 * be `true`.
			 *
			 * @type {?Array<Plane>}
			 * @default null
			 */
			this.clippingPlanes = null;

			/**
			 * Changes the behavior of clipping planes so that only their intersection is
			 * clipped, rather than their union.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.clipIntersection = false;

			/**
			 * Defines whether to clip shadows according to the clipping planes specified
			 * on this material.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.clipShadows = false;

			/**
			 * Defines which side of faces cast shadows. If `null`, the side casting shadows
			 * is determined as follows:
			 *
			 * - When {@link Material#side} is set to `FrontSide`, the back side cast shadows.
			 * - When {@link Material#side} is set to `BackSide`, the front side cast shadows.
			 * - When {@link Material#side} is set to `DoubleSide`, both sides cast shadows.
			 *
			 * @type {?(FrontSide|BackSide|DoubleSide)}
			 * @default null
			 */
			this.shadowSide = null;

			/**
			 * Whether to render the material's color.
			 *
			 * This can be used in conjunction with {@link Object3D#renderOder} to create invisible
			 * objects that occlude other objects.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.colorWrite = true;

			/**
			 * Override the renderer's default precision for this material.
			 *
			 * @type {?('highp'|'mediump'|'lowp')}
			 * @default null
			 */
			this.precision = null;

			/**
			 * Whether to use polygon offset or not. When enabled, each fragment's depth value will
			 * be offset after it is interpolated from the depth values of the appropriate vertices.
			 * The offset is added before the depth test is performed and before the value is written
			 * into the depth buffer.
			 *
			 * Can be useful for rendering hidden-line images, for applying decals to surfaces, and for
			 * rendering solids with highlighted edges.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.polygonOffset = false;

			/**
			 * Specifies a scale factor that is used to create a variable depth offset for each polygon.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.polygonOffsetFactor = 0;

			/**
			 * Is multiplied by an implementation-specific value to create a constant depth offset.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.polygonOffsetUnits = 0;

			/**
			 * Whether to apply dithering to the color to remove the appearance of banding.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.dithering = false;

			/**
			 * Whether alpha to coverage should be enabled or not. Can only be used with MSAA-enabled contexts
			 * (meaning when the renderer was created with *antialias* parameter set to `true`). Enabling this
			 * will smooth aliasing on clip plane edges and alphaTest-clipped edges.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.alphaToCoverage = false;

			/**
			 * Whether to premultiply the alpha (transparency) value.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.premultipliedAlpha = false;

			/**
			 * Whether double-sided, transparent objects should be rendered with a single pass or not.
			 *
			 * The engine renders double-sided, transparent objects with two draw calls (back faces first,
			 * then front faces) to mitigate transparency artifacts. There are scenarios however where this
			 * approach produces no quality gains but still doubles draw calls e.g. when rendering flat
			 * vegetation like grass sprites. In these cases, set the `forceSinglePass` flag to `true` to
			 * disable the two pass rendering to avoid performance issues.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.forceSinglePass = false;

			/**
			 * Whether it's possible to override the material with {@link Scene#overrideMaterial} or not.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.allowOverride = true;

			/**
			 * Defines whether 3D objects using this material are visible.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.visible = true;

			/**
			 * Defines whether this material is tone mapped according to the renderer's tone mapping setting.
			 *
			 * It is ignored when rendering to a render target or using post processing or when using
			 * `WebGPURenderer`. In all these cases, all materials are honored by tone mapping.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.toneMapped = true;

			/**
			 * An object that can be used to store custom data about the Material. It
			 * should not hold references to functions as these will not be cloned.
			 *
			 * @type {Object}
			 */
			this.userData = {};

			/**
			 * This starts at `0` and counts how many times {@link Material#needsUpdate} is set to `true`.
			 *
			 * @type {number}
			 * @readonly
			 * @default 0
			 */
			this.version = 0;

			this._alphaTest = 0;

		}

		/**
		 * Sets the alpha value to be used when running an alpha test. The material
		 * will not be rendered if the opacity is lower than this value.
		 *
		 * @type {number}
		 * @readonly
		 * @default 0
		 */
		get alphaTest() {

			return this._alphaTest;

		}

		set alphaTest( value ) {

			if ( this._alphaTest > 0 !== value > 0 ) {

				this.version ++;

			}

			this._alphaTest = value;

		}

		/**
		 * An optional callback that is executed immediately before the material is used to render a 3D object.
		 *
		 * This method can only be used when rendering with {@link WebGLRenderer}.
		 *
		 * @param {WebGLRenderer} renderer - The renderer.
		 * @param {Scene} scene - The scene.
		 * @param {Camera} camera - The camera that is used to render the scene.
		 * @param {BufferGeometry} geometry - The 3D object's geometry.
		 * @param {Object3D} object - The 3D object.
		 * @param {Object} group - The geometry group data.
		 */
		onBeforeRender( /* renderer, scene, camera, geometry, object, group */ ) {}

		/**
		 * An optional callback that is executed immediately before the shader
		 * program is compiled. This function is called with the shader source code
		 * as a parameter. Useful for the modification of built-in materials.
		 *
		 * This method can only be used when rendering with {@link WebGLRenderer}. The
		 * recommended approach when customizing materials is to use `WebGPURenderer` with the new
		 * Node Material system and [TSL]{@link https://github.com/mrdoob/three.js/wiki/Three.js-Shading-Language}.
		 *
		 * @param {{vertexShader:string,fragmentShader:string,uniforms:Object}} shaderobject - The object holds the uniforms and the vertex and fragment shader source.
		 * @param {WebGLRenderer} renderer - A reference to the renderer.
		 */
		onBeforeCompile( /* shaderobject, renderer */ ) {}

		/**
		 * In case {@link Material#onBeforeCompile} is used, this callback can be used to identify
		 * values of settings used in `onBeforeCompile()`, so three.js can reuse a cached
		 * shader or recompile the shader for this material as needed.
		 *
		 * This method can only be used when rendering with {@link WebGLRenderer}.
		 *
		 * @return {string} The custom program cache key.
		 */
		customProgramCacheKey() {

			return this.onBeforeCompile.toString();

		}

		/**
		 * This method can be used to set default values from parameter objects.
		 * It is a generic implementation so it can be used with different types
		 * of materials.
		 *
		 * @param {Object} [values] - The material values to set.
		 */
		setValues( values ) {

			if ( values === undefined ) return;

			for ( const key in values ) {

				const newValue = values[ key ];

				if ( newValue === undefined ) {

					console.warn( `THREE.Material: parameter '${ key }' has value of undefined.` );
					continue;

				}

				const currentValue = this[ key ];

				if ( currentValue === undefined ) {

					console.warn( `THREE.Material: '${ key }' is not a property of THREE.${ this.type }.` );
					continue;

				}

				if ( currentValue && currentValue.isColor ) {

					currentValue.set( newValue );

				} else if ( ( currentValue && currentValue.isVector3 ) && ( newValue && newValue.isVector3 ) ) {

					currentValue.copy( newValue );

				} else {

					this[ key ] = newValue;

				}

			}

		}

		/**
		 * Serializes the material into JSON.
		 *
		 * @param {?(Object|string)} meta - An optional value holding meta information about the serialization.
		 * @return {Object} A JSON object representing the serialized material.
		 * @see {@link ObjectLoader#parse}
		 */
		toJSON( meta ) {

			const isRootObject = ( meta === undefined || typeof meta === 'string' );

			if ( isRootObject ) {

				meta = {
					textures: {},
					images: {}
				};

			}

			const data = {
				metadata: {
					version: 4.7,
					type: 'Material',
					generator: 'Material.toJSON'
				}
			};

			// standard Material serialization
			data.uuid = this.uuid;
			data.type = this.type;

			if ( this.name !== '' ) data.name = this.name;

			if ( this.color && this.color.isColor ) data.color = this.color.getHex();

			if ( this.roughness !== undefined ) data.roughness = this.roughness;
			if ( this.metalness !== undefined ) data.metalness = this.metalness;

			if ( this.sheen !== undefined ) data.sheen = this.sheen;
			if ( this.sheenColor && this.sheenColor.isColor ) data.sheenColor = this.sheenColor.getHex();
			if ( this.sheenRoughness !== undefined ) data.sheenRoughness = this.sheenRoughness;
			if ( this.emissive && this.emissive.isColor ) data.emissive = this.emissive.getHex();
			if ( this.emissiveIntensity !== undefined && this.emissiveIntensity !== 1 ) data.emissiveIntensity = this.emissiveIntensity;

			if ( this.specular && this.specular.isColor ) data.specular = this.specular.getHex();
			if ( this.specularIntensity !== undefined ) data.specularIntensity = this.specularIntensity;
			if ( this.specularColor && this.specularColor.isColor ) data.specularColor = this.specularColor.getHex();
			if ( this.shininess !== undefined ) data.shininess = this.shininess;
			if ( this.clearcoat !== undefined ) data.clearcoat = this.clearcoat;
			if ( this.clearcoatRoughness !== undefined ) data.clearcoatRoughness = this.clearcoatRoughness;

			if ( this.clearcoatMap && this.clearcoatMap.isTexture ) {

				data.clearcoatMap = this.clearcoatMap.toJSON( meta ).uuid;

			}

			if ( this.clearcoatRoughnessMap && this.clearcoatRoughnessMap.isTexture ) {

				data.clearcoatRoughnessMap = this.clearcoatRoughnessMap.toJSON( meta ).uuid;

			}

			if ( this.clearcoatNormalMap && this.clearcoatNormalMap.isTexture ) {

				data.clearcoatNormalMap = this.clearcoatNormalMap.toJSON( meta ).uuid;
				data.clearcoatNormalScale = this.clearcoatNormalScale.toArray();

			}

			if ( this.sheenColorMap && this.sheenColorMap.isTexture ) {

				data.sheenColorMap = this.sheenColorMap.toJSON( meta ).uuid;

			}

			if ( this.sheenRoughnessMap && this.sheenRoughnessMap.isTexture ) {

				data.sheenRoughnessMap = this.sheenRoughnessMap.toJSON( meta ).uuid;

			}

			if ( this.dispersion !== undefined ) data.dispersion = this.dispersion;

			if ( this.iridescence !== undefined ) data.iridescence = this.iridescence;
			if ( this.iridescenceIOR !== undefined ) data.iridescenceIOR = this.iridescenceIOR;
			if ( this.iridescenceThicknessRange !== undefined ) data.iridescenceThicknessRange = this.iridescenceThicknessRange;

			if ( this.iridescenceMap && this.iridescenceMap.isTexture ) {

				data.iridescenceMap = this.iridescenceMap.toJSON( meta ).uuid;

			}

			if ( this.iridescenceThicknessMap && this.iridescenceThicknessMap.isTexture ) {

				data.iridescenceThicknessMap = this.iridescenceThicknessMap.toJSON( meta ).uuid;

			}

			if ( this.anisotropy !== undefined ) data.anisotropy = this.anisotropy;
			if ( this.anisotropyRotation !== undefined ) data.anisotropyRotation = this.anisotropyRotation;

			if ( this.anisotropyMap && this.anisotropyMap.isTexture ) {

				data.anisotropyMap = this.anisotropyMap.toJSON( meta ).uuid;

			}

			if ( this.map && this.map.isTexture ) data.map = this.map.toJSON( meta ).uuid;
			if ( this.matcap && this.matcap.isTexture ) data.matcap = this.matcap.toJSON( meta ).uuid;
			if ( this.alphaMap && this.alphaMap.isTexture ) data.alphaMap = this.alphaMap.toJSON( meta ).uuid;

			if ( this.lightMap && this.lightMap.isTexture ) {

				data.lightMap = this.lightMap.toJSON( meta ).uuid;
				data.lightMapIntensity = this.lightMapIntensity;

			}

			if ( this.aoMap && this.aoMap.isTexture ) {

				data.aoMap = this.aoMap.toJSON( meta ).uuid;
				data.aoMapIntensity = this.aoMapIntensity;

			}

			if ( this.bumpMap && this.bumpMap.isTexture ) {

				data.bumpMap = this.bumpMap.toJSON( meta ).uuid;
				data.bumpScale = this.bumpScale;

			}

			if ( this.normalMap && this.normalMap.isTexture ) {

				data.normalMap = this.normalMap.toJSON( meta ).uuid;
				data.normalMapType = this.normalMapType;
				data.normalScale = this.normalScale.toArray();

			}

			if ( this.displacementMap && this.displacementMap.isTexture ) {

				data.displacementMap = this.displacementMap.toJSON( meta ).uuid;
				data.displacementScale = this.displacementScale;
				data.displacementBias = this.displacementBias;

			}

			if ( this.roughnessMap && this.roughnessMap.isTexture ) data.roughnessMap = this.roughnessMap.toJSON( meta ).uuid;
			if ( this.metalnessMap && this.metalnessMap.isTexture ) data.metalnessMap = this.metalnessMap.toJSON( meta ).uuid;

			if ( this.emissiveMap && this.emissiveMap.isTexture ) data.emissiveMap = this.emissiveMap.toJSON( meta ).uuid;
			if ( this.specularMap && this.specularMap.isTexture ) data.specularMap = this.specularMap.toJSON( meta ).uuid;
			if ( this.specularIntensityMap && this.specularIntensityMap.isTexture ) data.specularIntensityMap = this.specularIntensityMap.toJSON( meta ).uuid;
			if ( this.specularColorMap && this.specularColorMap.isTexture ) data.specularColorMap = this.specularColorMap.toJSON( meta ).uuid;

			if ( this.envMap && this.envMap.isTexture ) {

				data.envMap = this.envMap.toJSON( meta ).uuid;

				if ( this.combine !== undefined ) data.combine = this.combine;

			}

			if ( this.envMapRotation !== undefined ) data.envMapRotation = this.envMapRotation.toArray();
			if ( this.envMapIntensity !== undefined ) data.envMapIntensity = this.envMapIntensity;
			if ( this.reflectivity !== undefined ) data.reflectivity = this.reflectivity;
			if ( this.refractionRatio !== undefined ) data.refractionRatio = this.refractionRatio;

			if ( this.gradientMap && this.gradientMap.isTexture ) {

				data.gradientMap = this.gradientMap.toJSON( meta ).uuid;

			}

			if ( this.transmission !== undefined ) data.transmission = this.transmission;
			if ( this.transmissionMap && this.transmissionMap.isTexture ) data.transmissionMap = this.transmissionMap.toJSON( meta ).uuid;
			if ( this.thickness !== undefined ) data.thickness = this.thickness;
			if ( this.thicknessMap && this.thicknessMap.isTexture ) data.thicknessMap = this.thicknessMap.toJSON( meta ).uuid;
			if ( this.attenuationDistance !== undefined && this.attenuationDistance !== Infinity ) data.attenuationDistance = this.attenuationDistance;
			if ( this.attenuationColor !== undefined ) data.attenuationColor = this.attenuationColor.getHex();

			if ( this.size !== undefined ) data.size = this.size;
			if ( this.shadowSide !== null ) data.shadowSide = this.shadowSide;
			if ( this.sizeAttenuation !== undefined ) data.sizeAttenuation = this.sizeAttenuation;

			if ( this.blending !== NormalBlending ) data.blending = this.blending;
			if ( this.side !== FrontSide ) data.side = this.side;
			if ( this.vertexColors === true ) data.vertexColors = true;

			if ( this.opacity < 1 ) data.opacity = this.opacity;
			if ( this.transparent === true ) data.transparent = true;

			if ( this.blendSrc !== SrcAlphaFactor ) data.blendSrc = this.blendSrc;
			if ( this.blendDst !== OneMinusSrcAlphaFactor ) data.blendDst = this.blendDst;
			if ( this.blendEquation !== AddEquation ) data.blendEquation = this.blendEquation;
			if ( this.blendSrcAlpha !== null ) data.blendSrcAlpha = this.blendSrcAlpha;
			if ( this.blendDstAlpha !== null ) data.blendDstAlpha = this.blendDstAlpha;
			if ( this.blendEquationAlpha !== null ) data.blendEquationAlpha = this.blendEquationAlpha;
			if ( this.blendColor && this.blendColor.isColor ) data.blendColor = this.blendColor.getHex();
			if ( this.blendAlpha !== 0 ) data.blendAlpha = this.blendAlpha;

			if ( this.depthFunc !== LessEqualDepth ) data.depthFunc = this.depthFunc;
			if ( this.depthTest === false ) data.depthTest = this.depthTest;
			if ( this.depthWrite === false ) data.depthWrite = this.depthWrite;
			if ( this.colorWrite === false ) data.colorWrite = this.colorWrite;

			if ( this.stencilWriteMask !== 0xff ) data.stencilWriteMask = this.stencilWriteMask;
			if ( this.stencilFunc !== AlwaysStencilFunc ) data.stencilFunc = this.stencilFunc;
			if ( this.stencilRef !== 0 ) data.stencilRef = this.stencilRef;
			if ( this.stencilFuncMask !== 0xff ) data.stencilFuncMask = this.stencilFuncMask;
			if ( this.stencilFail !== KeepStencilOp ) data.stencilFail = this.stencilFail;
			if ( this.stencilZFail !== KeepStencilOp ) data.stencilZFail = this.stencilZFail;
			if ( this.stencilZPass !== KeepStencilOp ) data.stencilZPass = this.stencilZPass;
			if ( this.stencilWrite === true ) data.stencilWrite = this.stencilWrite;

			// rotation (SpriteMaterial)
			if ( this.rotation !== undefined && this.rotation !== 0 ) data.rotation = this.rotation;

			if ( this.polygonOffset === true ) data.polygonOffset = true;
			if ( this.polygonOffsetFactor !== 0 ) data.polygonOffsetFactor = this.polygonOffsetFactor;
			if ( this.polygonOffsetUnits !== 0 ) data.polygonOffsetUnits = this.polygonOffsetUnits;

			if ( this.linewidth !== undefined && this.linewidth !== 1 ) data.linewidth = this.linewidth;
			if ( this.dashSize !== undefined ) data.dashSize = this.dashSize;
			if ( this.gapSize !== undefined ) data.gapSize = this.gapSize;
			if ( this.scale !== undefined ) data.scale = this.scale;

			if ( this.dithering === true ) data.dithering = true;

			if ( this.alphaTest > 0 ) data.alphaTest = this.alphaTest;
			if ( this.alphaHash === true ) data.alphaHash = true;
			if ( this.alphaToCoverage === true ) data.alphaToCoverage = true;
			if ( this.premultipliedAlpha === true ) data.premultipliedAlpha = true;
			if ( this.forceSinglePass === true ) data.forceSinglePass = true;

			if ( this.wireframe === true ) data.wireframe = true;
			if ( this.wireframeLinewidth > 1 ) data.wireframeLinewidth = this.wireframeLinewidth;
			if ( this.wireframeLinecap !== 'round' ) data.wireframeLinecap = this.wireframeLinecap;
			if ( this.wireframeLinejoin !== 'round' ) data.wireframeLinejoin = this.wireframeLinejoin;

			if ( this.flatShading === true ) data.flatShading = true;

			if ( this.visible === false ) data.visible = false;

			if ( this.toneMapped === false ) data.toneMapped = false;

			if ( this.fog === false ) data.fog = false;

			if ( Object.keys( this.userData ).length > 0 ) data.userData = this.userData;

			// TODO: Copied from Object3D.toJSON

			function extractFromCache( cache ) {

				const values = [];

				for ( const key in cache ) {

					const data = cache[ key ];
					delete data.metadata;
					values.push( data );

				}

				return values;

			}

			if ( isRootObject ) {

				const textures = extractFromCache( meta.textures );
				const images = extractFromCache( meta.images );

				if ( textures.length > 0 ) data.textures = textures;
				if ( images.length > 0 ) data.images = images;

			}

			return data;

		}

		/**
		 * Returns a new material with copied values from this instance.
		 *
		 * @return {Material} A clone of this instance.
		 */
		clone() {

			return new this.constructor().copy( this );

		}

		/**
		 * Copies the values of the given material to this instance.
		 *
		 * @param {Material} source - The material to copy.
		 * @return {Material} A reference to this instance.
		 */
		copy( source ) {

			this.name = source.name;

			this.blending = source.blending;
			this.side = source.side;
			this.vertexColors = source.vertexColors;

			this.opacity = source.opacity;
			this.transparent = source.transparent;

			this.blendSrc = source.blendSrc;
			this.blendDst = source.blendDst;
			this.blendEquation = source.blendEquation;
			this.blendSrcAlpha = source.blendSrcAlpha;
			this.blendDstAlpha = source.blendDstAlpha;
			this.blendEquationAlpha = source.blendEquationAlpha;
			this.blendColor.copy( source.blendColor );
			this.blendAlpha = source.blendAlpha;

			this.depthFunc = source.depthFunc;
			this.depthTest = source.depthTest;
			this.depthWrite = source.depthWrite;

			this.stencilWriteMask = source.stencilWriteMask;
			this.stencilFunc = source.stencilFunc;
			this.stencilRef = source.stencilRef;
			this.stencilFuncMask = source.stencilFuncMask;
			this.stencilFail = source.stencilFail;
			this.stencilZFail = source.stencilZFail;
			this.stencilZPass = source.stencilZPass;
			this.stencilWrite = source.stencilWrite;

			const srcPlanes = source.clippingPlanes;
			let dstPlanes = null;

			if ( srcPlanes !== null ) {

				const n = srcPlanes.length;
				dstPlanes = new Array( n );

				for ( let i = 0; i !== n; ++ i ) {

					dstPlanes[ i ] = srcPlanes[ i ].clone();

				}

			}

			this.clippingPlanes = dstPlanes;
			this.clipIntersection = source.clipIntersection;
			this.clipShadows = source.clipShadows;

			this.shadowSide = source.shadowSide;

			this.colorWrite = source.colorWrite;

			this.precision = source.precision;

			this.polygonOffset = source.polygonOffset;
			this.polygonOffsetFactor = source.polygonOffsetFactor;
			this.polygonOffsetUnits = source.polygonOffsetUnits;

			this.dithering = source.dithering;

			this.alphaTest = source.alphaTest;
			this.alphaHash = source.alphaHash;
			this.alphaToCoverage = source.alphaToCoverage;
			this.premultipliedAlpha = source.premultipliedAlpha;
			this.forceSinglePass = source.forceSinglePass;

			this.visible = source.visible;

			this.toneMapped = source.toneMapped;

			this.userData = JSON.parse( JSON.stringify( source.userData ) );

			return this;

		}

		/**
		 * Frees the GPU-related resources allocated by this instance. Call this
		 * method whenever this instance is no longer used in your app.
		 *
		 * @fires Material#dispose
		 */
		dispose() {

			/**
			 * Fires when the material has been disposed of.
			 *
			 * @event Material#dispose
			 * @type {Object}
			 */
			this.dispatchEvent( { type: 'dispose' } );

		}

		/**
		 * Setting this property to `true` indicates the engine the material
		 * needs to be recompiled.
		 *
		 * @type {boolean}
		 * @default false
		 * @param {boolean} value
		 */
		set needsUpdate( value ) {

			if ( value === true ) this.version ++;

		}

	}

	/**
	 * A material for drawing geometries in a simple shaded (flat or wireframe) way.
	 *
	 * This material is not affected by lights.
	 *
	 * @augments Material
	 */
	class MeshBasicMaterial extends Material {

		/**
		 * Constructs a new mesh basic material.
		 *
		 * @param {Object} [parameters] - An object with one or more properties
		 * defining the material's appearance. Any property of the material
		 * (including any property from inherited materials) can be passed
		 * in here. Color values can be passed any type of value accepted
		 * by {@link Color#set}.
		 */
		constructor( parameters ) {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isMeshBasicMaterial = true;

			this.type = 'MeshBasicMaterial';

			/**
			 * Color of the material.
			 *
			 * @type {Color}
			 * @default (1,1,1)
			 */
			this.color = new Color( 0xffffff ); // diffuse

			/**
			 * The color map. May optionally include an alpha channel, typically combined
			 * with {@link Material#transparent} or {@link Material#alphaTest}. The texture map
			 * color is modulated by the diffuse `color`.
			 *
			 * @type {?Texture}
			 * @default null
			 */
			this.map = null;

			/**
			 * The light map. Requires a second set of UVs.
			 *
			 * @type {?Texture}
			 * @default null
			 */
			this.lightMap = null;

			/**
			 * Intensity of the baked light.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.lightMapIntensity = 1.0;

			/**
			 * The red channel of this texture is used as the ambient occlusion map.
			 * Requires a second set of UVs.
			 *
			 * @type {?Texture}
			 * @default null
			 */
			this.aoMap = null;

			/**
			 * Intensity of the ambient occlusion effect. Range is `[0,1]`, where `0`
			 * disables ambient occlusion. Where intensity is `1` and the AO map's
			 * red channel is also `1`, ambient light is fully occluded on a surface.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.aoMapIntensity = 1.0;

			/**
			 * Specular map used by the material.
			 *
			 * @type {?Texture}
			 * @default null
			 */
			this.specularMap = null;

			/**
			 * The alpha map is a grayscale texture that controls the opacity across the
			 * surface (black: fully transparent; white: fully opaque).
			 *
			 * Only the color of the texture is used, ignoring the alpha channel if one
			 * exists. For RGB and RGBA textures, the renderer will use the green channel
			 * when sampling this texture due to the extra bit of precision provided for
			 * green in DXT-compressed and uncompressed RGB 565 formats. Luminance-only and
			 * luminance/alpha textures will also still work as expected.
			 *
			 * @type {?Texture}
			 * @default null
			 */
			this.alphaMap = null;

			/**
			 * The environment map.
			 *
			 * @type {?Texture}
			 * @default null
			 */
			this.envMap = null;

			/**
			 * The rotation of the environment map in radians.
			 *
			 * @type {Euler}
			 * @default (0,0,0)
			 */
			this.envMapRotation = new Euler();

			/**
			 * How to combine the result of the surface's color with the environment map, if any.
			 *
			 * When set to `MixOperation`, the {@link MeshBasicMaterial#reflectivity} is used to
			 * blend between the two colors.
			 *
			 * @type {(MultiplyOperation|MixOperation|AddOperation)}
			 * @default MultiplyOperation
			 */
			this.combine = MultiplyOperation;

			/**
			 * How much the environment map affects the surface.
			 * The valid range is between `0` (no reflections) and `1` (full reflections).
			 *
			 * @type {number}
			 * @default 1
			 */
			this.reflectivity = 1;

			/**
			 * The index of refraction (IOR) of air (approximately 1) divided by the
			 * index of refraction of the material. It is used with environment mapping
			 * modes {@link CubeRefractionMapping} and {@link EquirectangularRefractionMapping}.
			 * The refraction ratio should not exceed `1`.
			 *
			 * @type {number}
			 * @default 0.98
			 */
			this.refractionRatio = 0.98;

			/**
			 * Renders the geometry as a wireframe.
			 *
			 * @type {boolean}
			 * @default false
			 */
			this.wireframe = false;

			/**
			 * Controls the thickness of the wireframe.
			 *
			 * Can only be used with {@link SVGRenderer}.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.wireframeLinewidth = 1;

			/**
			 * Defines appearance of wireframe ends.
			 *
			 * Can only be used with {@link SVGRenderer}.
			 *
			 * @type {('round'|'bevel'|'miter')}
			 * @default 'round'
			 */
			this.wireframeLinecap = 'round';

			/**
			 * Defines appearance of wireframe joints.
			 *
			 * Can only be used with {@link SVGRenderer}.
			 *
			 * @type {('round'|'bevel'|'miter')}
			 * @default 'round'
			 */
			this.wireframeLinejoin = 'round';

			/**
			 * Whether the material is affected by fog or not.
			 *
			 * @type {boolean}
			 * @default true
			 */
			this.fog = true;

			this.setValues( parameters );

		}

		copy( source ) {

			super.copy( source );

			this.color.copy( source.color );

			this.map = source.map;

			this.lightMap = source.lightMap;
			this.lightMapIntensity = source.lightMapIntensity;

			this.aoMap = source.aoMap;
			this.aoMapIntensity = source.aoMapIntensity;

			this.specularMap = source.specularMap;

			this.alphaMap = source.alphaMap;

			this.envMap = source.envMap;
			this.envMapRotation.copy( source.envMapRotation );
			this.combine = source.combine;
			this.reflectivity = source.reflectivity;
			this.refractionRatio = source.refractionRatio;

			this.wireframe = source.wireframe;
			this.wireframeLinewidth = source.wireframeLinewidth;
			this.wireframeLinecap = source.wireframeLinecap;
			this.wireframeLinejoin = source.wireframeLinejoin;

			this.fog = source.fog;

			return this;

		}

	}

	const _vector = /*@__PURE__*/ new Vector3();
	const _segCenter = /*@__PURE__*/ new Vector3();
	const _segDir = /*@__PURE__*/ new Vector3();
	const _diff = /*@__PURE__*/ new Vector3();

	const _edge1 = /*@__PURE__*/ new Vector3();
	const _edge2 = /*@__PURE__*/ new Vector3();
	const _normal = /*@__PURE__*/ new Vector3();

	/**
	 * A ray that emits from an origin in a certain direction. The class is used by
	 * {@link Raycaster} to assist with raycasting. Raycasting is used for
	 * mouse picking (working out what objects in the 3D space the mouse is over)
	 * amongst other things.
	 */
	class Ray {

		/**
		 * Constructs a new ray.
		 *
		 * @param {Vector3} [origin=(0,0,0)] - The origin of the ray.
		 * @param {Vector3} [direction=(0,0,-1)] - The (normalized) direction of the ray.
		 */
		constructor( origin = new Vector3(), direction = new Vector3( 0, 0, - 1 ) ) {

			/**
			 * The origin of the ray.
			 *
			 * @type {Vector3}
			 */
			this.origin = origin;

			/**
			 * The (normalized) direction of the ray.
			 *
			 * @type {Vector3}
			 */
			this.direction = direction;

		}

		/**
		 * Sets the ray's components by copying the given values.
		 *
		 * @param {Vector3} origin - The origin.
		 * @param {Vector3} direction - The direction.
		 * @return {Ray} A reference to this ray.
		 */
		set( origin, direction ) {

			this.origin.copy( origin );
			this.direction.copy( direction );

			return this;

		}

		/**
		 * Copies the values of the given ray to this instance.
		 *
		 * @param {Ray} ray - The ray to copy.
		 * @return {Ray} A reference to this ray.
		 */
		copy( ray ) {

			this.origin.copy( ray.origin );
			this.direction.copy( ray.direction );

			return this;

		}

		/**
		 * Returns a vector that is located at a given distance along this ray.
		 *
		 * @param {number} t - The distance along the ray to retrieve a position for.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} A position on the ray.
		 */
		at( t, target ) {

			return target.copy( this.origin ).addScaledVector( this.direction, t );

		}

		/**
		 * Adjusts the direction of the ray to point at the given vector in world space.
		 *
		 * @param {Vector3} v - The target position.
		 * @return {Ray} A reference to this ray.
		 */
		lookAt( v ) {

			this.direction.copy( v ).sub( this.origin ).normalize();

			return this;

		}

		/**
		 * Shift the origin of this ray along its direction by the given distance.
		 *
		 * @param {number} t - The distance along the ray to interpolate.
		 * @return {Ray} A reference to this ray.
		 */
		recast( t ) {

			this.origin.copy( this.at( t, _vector ) );

			return this;

		}

		/**
		 * Returns the point along this ray that is closest to the given point.
		 *
		 * @param {Vector3} point - A point in 3D space to get the closet location on the ray for.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The closest point on this ray.
		 */
		closestPointToPoint( point, target ) {

			target.subVectors( point, this.origin );

			const directionDistance = target.dot( this.direction );

			if ( directionDistance < 0 ) {

				return target.copy( this.origin );

			}

			return target.copy( this.origin ).addScaledVector( this.direction, directionDistance );

		}

		/**
		 * Returns the distance of the closest approach between this ray and the given point.
		 *
		 * @param {Vector3} point - A point in 3D space to compute the distance to.
		 * @return {number} The distance.
		 */
		distanceToPoint( point ) {

			return Math.sqrt( this.distanceSqToPoint( point ) );

		}

		/**
		 * Returns the squared distance of the closest approach between this ray and the given point.
		 *
		 * @param {Vector3} point - A point in 3D space to compute the distance to.
		 * @return {number} The squared distance.
		 */
		distanceSqToPoint( point ) {

			const directionDistance = _vector.subVectors( point, this.origin ).dot( this.direction );

			// point behind the ray

			if ( directionDistance < 0 ) {

				return this.origin.distanceToSquared( point );

			}

			_vector.copy( this.origin ).addScaledVector( this.direction, directionDistance );

			return _vector.distanceToSquared( point );

		}

		/**
		 * Returns the squared distance between this ray and the given line segment.
		 *
		 * @param {Vector3} v0 - The start point of the line segment.
		 * @param {Vector3} v1 - The end point of the line segment.
		 * @param {Vector3} [optionalPointOnRay] - When provided, it receives the point on this ray that is closest to the segment.
		 * @param {Vector3} [optionalPointOnSegment] - When provided, it receives the point on the line segment that is closest to this ray.
		 * @return {number} The squared distance.
		 */
		distanceSqToSegment( v0, v1, optionalPointOnRay, optionalPointOnSegment ) {

			// from https://github.com/pmjoniak/GeometricTools/blob/master/GTEngine/Include/Mathematics/GteDistRaySegment.h
			// It returns the min distance between the ray and the segment
			// defined by v0 and v1
			// It can also set two optional targets :
			// - The closest point on the ray
			// - The closest point on the segment

			_segCenter.copy( v0 ).add( v1 ).multiplyScalar( 0.5 );
			_segDir.copy( v1 ).sub( v0 ).normalize();
			_diff.copy( this.origin ).sub( _segCenter );

			const segExtent = v0.distanceTo( v1 ) * 0.5;
			const a01 = - this.direction.dot( _segDir );
			const b0 = _diff.dot( this.direction );
			const b1 = - _diff.dot( _segDir );
			const c = _diff.lengthSq();
			const det = Math.abs( 1 - a01 * a01 );
			let s0, s1, sqrDist, extDet;

			if ( det > 0 ) {

				// The ray and segment are not parallel.

				s0 = a01 * b1 - b0;
				s1 = a01 * b0 - b1;
				extDet = segExtent * det;

				if ( s0 >= 0 ) {

					if ( s1 >= - extDet ) {

						if ( s1 <= extDet ) {

							// region 0
							// Minimum at interior points of ray and segment.

							const invDet = 1 / det;
							s0 *= invDet;
							s1 *= invDet;
							sqrDist = s0 * ( s0 + a01 * s1 + 2 * b0 ) + s1 * ( a01 * s0 + s1 + 2 * b1 ) + c;

						} else {

							// region 1

							s1 = segExtent;
							s0 = Math.max( 0, - ( a01 * s1 + b0 ) );
							sqrDist = - s0 * s0 + s1 * ( s1 + 2 * b1 ) + c;

						}

					} else {

						// region 5

						s1 = - segExtent;
						s0 = Math.max( 0, - ( a01 * s1 + b0 ) );
						sqrDist = - s0 * s0 + s1 * ( s1 + 2 * b1 ) + c;

					}

				} else {

					if ( s1 <= - extDet ) {

						// region 4

						s0 = Math.max( 0, - ( - a01 * segExtent + b0 ) );
						s1 = ( s0 > 0 ) ? - segExtent : Math.min( Math.max( - segExtent, - b1 ), segExtent );
						sqrDist = - s0 * s0 + s1 * ( s1 + 2 * b1 ) + c;

					} else if ( s1 <= extDet ) {

						// region 3

						s0 = 0;
						s1 = Math.min( Math.max( - segExtent, - b1 ), segExtent );
						sqrDist = s1 * ( s1 + 2 * b1 ) + c;

					} else {

						// region 2

						s0 = Math.max( 0, - ( a01 * segExtent + b0 ) );
						s1 = ( s0 > 0 ) ? segExtent : Math.min( Math.max( - segExtent, - b1 ), segExtent );
						sqrDist = - s0 * s0 + s1 * ( s1 + 2 * b1 ) + c;

					}

				}

			} else {

				// Ray and segment are parallel.

				s1 = ( a01 > 0 ) ? - segExtent : segExtent;
				s0 = Math.max( 0, - ( a01 * s1 + b0 ) );
				sqrDist = - s0 * s0 + s1 * ( s1 + 2 * b1 ) + c;

			}

			if ( optionalPointOnRay ) {

				optionalPointOnRay.copy( this.origin ).addScaledVector( this.direction, s0 );

			}

			if ( optionalPointOnSegment ) {

				optionalPointOnSegment.copy( _segCenter ).addScaledVector( _segDir, s1 );

			}

			return sqrDist;

		}

		/**
		 * Intersects this ray with the given sphere, returning the intersection
		 * point or `null` if there is no intersection.
		 *
		 * @param {Sphere} sphere - The sphere to intersect.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The intersection point.
		 */
		intersectSphere( sphere, target ) {

			_vector.subVectors( sphere.center, this.origin );
			const tca = _vector.dot( this.direction );
			const d2 = _vector.dot( _vector ) - tca * tca;
			const radius2 = sphere.radius * sphere.radius;

			if ( d2 > radius2 ) return null;

			const thc = Math.sqrt( radius2 - d2 );

			// t0 = first intersect point - entrance on front of sphere
			const t0 = tca - thc;

			// t1 = second intersect point - exit point on back of sphere
			const t1 = tca + thc;

			// test to see if t1 is behind the ray - if so, return null
			if ( t1 < 0 ) return null;

			// test to see if t0 is behind the ray:
			// if it is, the ray is inside the sphere, so return the second exit point scaled by t1,
			// in order to always return an intersect point that is in front of the ray.
			if ( t0 < 0 ) return this.at( t1, target );

			// else t0 is in front of the ray, so return the first collision point scaled by t0
			return this.at( t0, target );

		}

		/**
		 * Returns `true` if this ray intersects with the given sphere.
		 *
		 * @param {Sphere} sphere - The sphere to intersect.
		 * @return {boolean} Whether this ray intersects with the given sphere or not.
		 */
		intersectsSphere( sphere ) {

			if ( sphere.radius < 0 ) return false; // handle empty spheres, see #31187

			return this.distanceSqToPoint( sphere.center ) <= ( sphere.radius * sphere.radius );

		}

		/**
		 * Computes the distance from the ray's origin to the given plane. Returns `null` if the ray
		 * does not intersect with the plane.
		 *
		 * @param {Plane} plane - The plane to compute the distance to.
		 * @return {?number} Whether this ray intersects with the given sphere or not.
		 */
		distanceToPlane( plane ) {

			const denominator = plane.normal.dot( this.direction );

			if ( denominator === 0 ) {

				// line is coplanar, return origin
				if ( plane.distanceToPoint( this.origin ) === 0 ) {

					return 0;

				}

				// Null is preferable to undefined since undefined means.... it is undefined

				return null;

			}

			const t = - ( this.origin.dot( plane.normal ) + plane.constant ) / denominator;

			// Return if the ray never intersects the plane

			return t >= 0 ? t : null;

		}

		/**
		 * Intersects this ray with the given plane, returning the intersection
		 * point or `null` if there is no intersection.
		 *
		 * @param {Plane} plane - The plane to intersect.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The intersection point.
		 */
		intersectPlane( plane, target ) {

			const t = this.distanceToPlane( plane );

			if ( t === null ) {

				return null;

			}

			return this.at( t, target );

		}

		/**
		 * Returns `true` if this ray intersects with the given plane.
		 *
		 * @param {Plane} plane - The plane to intersect.
		 * @return {boolean} Whether this ray intersects with the given plane or not.
		 */
		intersectsPlane( plane ) {

			// check if the ray lies on the plane first

			const distToPoint = plane.distanceToPoint( this.origin );

			if ( distToPoint === 0 ) {

				return true;

			}

			const denominator = plane.normal.dot( this.direction );

			if ( denominator * distToPoint < 0 ) {

				return true;

			}

			// ray origin is behind the plane (and is pointing behind it)

			return false;

		}

		/**
		 * Intersects this ray with the given bounding box, returning the intersection
		 * point or `null` if there is no intersection.
		 *
		 * @param {Box3} box - The box to intersect.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The intersection point.
		 */
		intersectBox( box, target ) {

			let tmin, tmax, tymin, tymax, tzmin, tzmax;

			const invdirx = 1 / this.direction.x,
				invdiry = 1 / this.direction.y,
				invdirz = 1 / this.direction.z;

			const origin = this.origin;

			if ( invdirx >= 0 ) {

				tmin = ( box.min.x - origin.x ) * invdirx;
				tmax = ( box.max.x - origin.x ) * invdirx;

			} else {

				tmin = ( box.max.x - origin.x ) * invdirx;
				tmax = ( box.min.x - origin.x ) * invdirx;

			}

			if ( invdiry >= 0 ) {

				tymin = ( box.min.y - origin.y ) * invdiry;
				tymax = ( box.max.y - origin.y ) * invdiry;

			} else {

				tymin = ( box.max.y - origin.y ) * invdiry;
				tymax = ( box.min.y - origin.y ) * invdiry;

			}

			if ( ( tmin > tymax ) || ( tymin > tmax ) ) return null;

			if ( tymin > tmin || isNaN( tmin ) ) tmin = tymin;

			if ( tymax < tmax || isNaN( tmax ) ) tmax = tymax;

			if ( invdirz >= 0 ) {

				tzmin = ( box.min.z - origin.z ) * invdirz;
				tzmax = ( box.max.z - origin.z ) * invdirz;

			} else {

				tzmin = ( box.max.z - origin.z ) * invdirz;
				tzmax = ( box.min.z - origin.z ) * invdirz;

			}

			if ( ( tmin > tzmax ) || ( tzmin > tmax ) ) return null;

			if ( tzmin > tmin || tmin !== tmin ) tmin = tzmin;

			if ( tzmax < tmax || tmax !== tmax ) tmax = tzmax;

			//return point closest to the ray (positive side)

			if ( tmax < 0 ) return null;

			return this.at( tmin >= 0 ? tmin : tmax, target );

		}

		/**
		 * Returns `true` if this ray intersects with the given box.
		 *
		 * @param {Box3} box - The box to intersect.
		 * @return {boolean} Whether this ray intersects with the given box or not.
		 */
		intersectsBox( box ) {

			return this.intersectBox( box, _vector ) !== null;

		}

		/**
		 * Intersects this ray with the given triangle, returning the intersection
		 * point or `null` if there is no intersection.
		 *
		 * @param {Vector3} a - The first vertex of the triangle.
		 * @param {Vector3} b - The second vertex of the triangle.
		 * @param {Vector3} c - The third vertex of the triangle.
		 * @param {boolean} backfaceCulling - Whether to use backface culling or not.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The intersection point.
		 */
		intersectTriangle( a, b, c, backfaceCulling, target ) {

			// Compute the offset origin, edges, and normal.

			// from https://github.com/pmjoniak/GeometricTools/blob/master/GTEngine/Include/Mathematics/GteIntrRay3Triangle3.h

			_edge1.subVectors( b, a );
			_edge2.subVectors( c, a );
			_normal.crossVectors( _edge1, _edge2 );

			// Solve Q + t*D = b1*E1 + b2*E2 (Q = kDiff, D = ray direction,
			// E1 = kEdge1, E2 = kEdge2, N = Cross(E1,E2)) by
			//   |Dot(D,N)|*b1 = sign(Dot(D,N))*Dot(D,Cross(Q,E2))
			//   |Dot(D,N)|*b2 = sign(Dot(D,N))*Dot(D,Cross(E1,Q))
			//   |Dot(D,N)|*t = -sign(Dot(D,N))*Dot(Q,N)
			let DdN = this.direction.dot( _normal );
			let sign;

			if ( DdN > 0 ) {

				if ( backfaceCulling ) return null;
				sign = 1;

			} else if ( DdN < 0 ) {

				sign = - 1;
				DdN = - DdN;

			} else {

				return null;

			}

			_diff.subVectors( this.origin, a );
			const DdQxE2 = sign * this.direction.dot( _edge2.crossVectors( _diff, _edge2 ) );

			// b1 < 0, no intersection
			if ( DdQxE2 < 0 ) {

				return null;

			}

			const DdE1xQ = sign * this.direction.dot( _edge1.cross( _diff ) );

			// b2 < 0, no intersection
			if ( DdE1xQ < 0 ) {

				return null;

			}

			// b1+b2 > 1, no intersection
			if ( DdQxE2 + DdE1xQ > DdN ) {

				return null;

			}

			// Line intersects triangle, check if ray does.
			const QdN = - sign * _diff.dot( _normal );

			// t < 0, no intersection
			if ( QdN < 0 ) {

				return null;

			}

			// Ray intersects triangle.
			return this.at( QdN / DdN, target );

		}

		/**
		 * Transforms this ray with the given 4x4 transformation matrix.
		 *
		 * @param {Matrix4} matrix4 - The transformation matrix.
		 * @return {Ray} A reference to this ray.
		 */
		applyMatrix4( matrix4 ) {

			this.origin.applyMatrix4( matrix4 );
			this.direction.transformDirection( matrix4 );

			return this;

		}

		/**
		 * Returns `true` if this ray is equal with the given one.
		 *
		 * @param {Ray} ray - The ray to test for equality.
		 * @return {boolean} Whether this ray is equal with the given one.
		 */
		equals( ray ) {

			return ray.origin.equals( this.origin ) && ray.direction.equals( this.direction );

		}

		/**
		 * Returns a new ray with copied values from this instance.
		 *
		 * @return {Ray} A clone of this instance.
		 */
		clone() {

			return new this.constructor().copy( this );

		}

	}

	/**
	 * Class representing a 4D vector. A 4D vector is an ordered quadruplet of numbers
	 * (labeled x, y, z and w), which can be used to represent a number of things, such as:
	 *
	 * - A point in 4D space.
	 * - A direction and length in 4D space. In three.js the length will
	 * always be the Euclidean distance(straight-line distance) from `(0, 0, 0, 0)` to `(x, y, z, w)`
	 * and the direction is also measured from `(0, 0, 0, 0)` towards `(x, y, z, w)`.
	 * - Any arbitrary ordered quadruplet of numbers.
	 *
	 * There are other things a 4D vector can be used to represent, however these
	 * are the most common uses in *three.js*.
	 *
	 * Iterating through a vector instance will yield its components `(x, y, z, w)` in
	 * the corresponding order.
	 * ```js
	 * const a = new THREE.Vector4( 0, 1, 0, 0 );
	 *
	 * //no arguments; will be initialised to (0, 0, 0, 1)
	 * const b = new THREE.Vector4( );
	 *
	 * const d = a.dot( b );
	 * ```
	 */
	class Vector4 {

		/**
		 * Constructs a new 4D vector.
		 *
		 * @param {number} [x=0] - The x value of this vector.
		 * @param {number} [y=0] - The y value of this vector.
		 * @param {number} [z=0] - The z value of this vector.
		 * @param {number} [w=1] - The w value of this vector.
		 */
		constructor( x = 0, y = 0, z = 0, w = 1 ) {

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			Vector4.prototype.isVector4 = true;

			/**
			 * The x value of this vector.
			 *
			 * @type {number}
			 */
			this.x = x;

			/**
			 * The y value of this vector.
			 *
			 * @type {number}
			 */
			this.y = y;

			/**
			 * The z value of this vector.
			 *
			 * @type {number}
			 */
			this.z = z;

			/**
			 * The w value of this vector.
			 *
			 * @type {number}
			 */
			this.w = w;

		}

		/**
		 * Alias for {@link Vector4#z}.
		 *
		 * @type {number}
		 */
		get width() {

			return this.z;

		}

		set width( value ) {

			this.z = value;

		}

		/**
		 * Alias for {@link Vector4#w}.
		 *
		 * @type {number}
		 */
		get height() {

			return this.w;

		}

		set height( value ) {

			this.w = value;

		}

		/**
		 * Sets the vector components.
		 *
		 * @param {number} x - The value of the x component.
		 * @param {number} y - The value of the y component.
		 * @param {number} z - The value of the z component.
		 * @param {number} w - The value of the w component.
		 * @return {Vector4} A reference to this vector.
		 */
		set( x, y, z, w ) {

			this.x = x;
			this.y = y;
			this.z = z;
			this.w = w;

			return this;

		}

		/**
		 * Sets the vector components to the same value.
		 *
		 * @param {number} scalar - The value to set for all vector components.
		 * @return {Vector4} A reference to this vector.
		 */
		setScalar( scalar ) {

			this.x = scalar;
			this.y = scalar;
			this.z = scalar;
			this.w = scalar;

			return this;

		}

		/**
		 * Sets the vector's x component to the given value
		 *
		 * @param {number} x - The value to set.
		 * @return {Vector4} A reference to this vector.
		 */
		setX( x ) {

			this.x = x;

			return this;

		}

		/**
		 * Sets the vector's y component to the given value
		 *
		 * @param {number} y - The value to set.
		 * @return {Vector4} A reference to this vector.
		 */
		setY( y ) {

			this.y = y;

			return this;

		}

		/**
		 * Sets the vector's z component to the given value
		 *
		 * @param {number} z - The value to set.
		 * @return {Vector4} A reference to this vector.
		 */
		setZ( z ) {

			this.z = z;

			return this;

		}

		/**
		 * Sets the vector's w component to the given value
		 *
		 * @param {number} w - The value to set.
		 * @return {Vector4} A reference to this vector.
		 */
		setW( w ) {

			this.w = w;

			return this;

		}

		/**
		 * Allows to set a vector component with an index.
		 *
		 * @param {number} index - The component index. `0` equals to x, `1` equals to y,
		 * `2` equals to z, `3` equals to w.
		 * @param {number} value - The value to set.
		 * @return {Vector4} A reference to this vector.
		 */
		setComponent( index, value ) {

			switch ( index ) {

				case 0: this.x = value; break;
				case 1: this.y = value; break;
				case 2: this.z = value; break;
				case 3: this.w = value; break;
				default: throw new Error( 'index is out of range: ' + index );

			}

			return this;

		}

		/**
		 * Returns the value of the vector component which matches the given index.
		 *
		 * @param {number} index - The component index. `0` equals to x, `1` equals to y,
		 * `2` equals to z, `3` equals to w.
		 * @return {number} A vector component value.
		 */
		getComponent( index ) {

			switch ( index ) {

				case 0: return this.x;
				case 1: return this.y;
				case 2: return this.z;
				case 3: return this.w;
				default: throw new Error( 'index is out of range: ' + index );

			}

		}

		/**
		 * Returns a new vector with copied values from this instance.
		 *
		 * @return {Vector4} A clone of this instance.
		 */
		clone() {

			return new this.constructor( this.x, this.y, this.z, this.w );

		}

		/**
		 * Copies the values of the given vector to this instance.
		 *
		 * @param {Vector3|Vector4} v - The vector to copy.
		 * @return {Vector4} A reference to this vector.
		 */
		copy( v ) {

			this.x = v.x;
			this.y = v.y;
			this.z = v.z;
			this.w = ( v.w !== undefined ) ? v.w : 1;

			return this;

		}

		/**
		 * Adds the given vector to this instance.
		 *
		 * @param {Vector4} v - The vector to add.
		 * @return {Vector4} A reference to this vector.
		 */
		add( v ) {

			this.x += v.x;
			this.y += v.y;
			this.z += v.z;
			this.w += v.w;

			return this;

		}

		/**
		 * Adds the given scalar value to all components of this instance.
		 *
		 * @param {number} s - The scalar to add.
		 * @return {Vector4} A reference to this vector.
		 */
		addScalar( s ) {

			this.x += s;
			this.y += s;
			this.z += s;
			this.w += s;

			return this;

		}

		/**
		 * Adds the given vectors and stores the result in this instance.
		 *
		 * @param {Vector4} a - The first vector.
		 * @param {Vector4} b - The second vector.
		 * @return {Vector4} A reference to this vector.
		 */
		addVectors( a, b ) {

			this.x = a.x + b.x;
			this.y = a.y + b.y;
			this.z = a.z + b.z;
			this.w = a.w + b.w;

			return this;

		}

		/**
		 * Adds the given vector scaled by the given factor to this instance.
		 *
		 * @param {Vector4} v - The vector.
		 * @param {number} s - The factor that scales `v`.
		 * @return {Vector4} A reference to this vector.
		 */
		addScaledVector( v, s ) {

			this.x += v.x * s;
			this.y += v.y * s;
			this.z += v.z * s;
			this.w += v.w * s;

			return this;

		}

		/**
		 * Subtracts the given vector from this instance.
		 *
		 * @param {Vector4} v - The vector to subtract.
		 * @return {Vector4} A reference to this vector.
		 */
		sub( v ) {

			this.x -= v.x;
			this.y -= v.y;
			this.z -= v.z;
			this.w -= v.w;

			return this;

		}

		/**
		 * Subtracts the given scalar value from all components of this instance.
		 *
		 * @param {number} s - The scalar to subtract.
		 * @return {Vector4} A reference to this vector.
		 */
		subScalar( s ) {

			this.x -= s;
			this.y -= s;
			this.z -= s;
			this.w -= s;

			return this;

		}

		/**
		 * Subtracts the given vectors and stores the result in this instance.
		 *
		 * @param {Vector4} a - The first vector.
		 * @param {Vector4} b - The second vector.
		 * @return {Vector4} A reference to this vector.
		 */
		subVectors( a, b ) {

			this.x = a.x - b.x;
			this.y = a.y - b.y;
			this.z = a.z - b.z;
			this.w = a.w - b.w;

			return this;

		}

		/**
		 * Multiplies the given vector with this instance.
		 *
		 * @param {Vector4} v - The vector to multiply.
		 * @return {Vector4} A reference to this vector.
		 */
		multiply( v ) {

			this.x *= v.x;
			this.y *= v.y;
			this.z *= v.z;
			this.w *= v.w;

			return this;

		}

		/**
		 * Multiplies the given scalar value with all components of this instance.
		 *
		 * @param {number} scalar - The scalar to multiply.
		 * @return {Vector4} A reference to this vector.
		 */
		multiplyScalar( scalar ) {

			this.x *= scalar;
			this.y *= scalar;
			this.z *= scalar;
			this.w *= scalar;

			return this;

		}

		/**
		 * Multiplies this vector with the given 4x4 matrix.
		 *
		 * @param {Matrix4} m - The 4x4 matrix.
		 * @return {Vector4} A reference to this vector.
		 */
		applyMatrix4( m ) {

			const x = this.x, y = this.y, z = this.z, w = this.w;
			const e = m.elements;

			this.x = e[ 0 ] * x + e[ 4 ] * y + e[ 8 ] * z + e[ 12 ] * w;
			this.y = e[ 1 ] * x + e[ 5 ] * y + e[ 9 ] * z + e[ 13 ] * w;
			this.z = e[ 2 ] * x + e[ 6 ] * y + e[ 10 ] * z + e[ 14 ] * w;
			this.w = e[ 3 ] * x + e[ 7 ] * y + e[ 11 ] * z + e[ 15 ] * w;

			return this;

		}

		/**
		 * Divides this instance by the given vector.
		 *
		 * @param {Vector4} v - The vector to divide.
		 * @return {Vector4} A reference to this vector.
		 */
		divide( v ) {

			this.x /= v.x;
			this.y /= v.y;
			this.z /= v.z;
			this.w /= v.w;

			return this;

		}

		/**
		 * Divides this vector by the given scalar.
		 *
		 * @param {number} scalar - The scalar to divide.
		 * @return {Vector4} A reference to this vector.
		 */
		divideScalar( scalar ) {

			return this.multiplyScalar( 1 / scalar );

		}

		/**
		 * Sets the x, y and z components of this
		 * vector to the quaternion's axis and w to the angle.
		 *
		 * @param {Quaternion} q - The Quaternion to set.
		 * @return {Vector4} A reference to this vector.
		 */
		setAxisAngleFromQuaternion( q ) {

			// http://www.euclideanspace.com/maths/geometry/rotations/conversions/quaternionToAngle/index.htm

			// q is assumed to be normalized

			this.w = 2 * Math.acos( q.w );

			const s = Math.sqrt( 1 - q.w * q.w );

			if ( s < 0.0001 ) {

				this.x = 1;
				this.y = 0;
				this.z = 0;

			} else {

				this.x = q.x / s;
				this.y = q.y / s;
				this.z = q.z / s;

			}

			return this;

		}

		/**
		 * Sets the x, y and z components of this
		 * vector to the axis of rotation and w to the angle.
		 *
		 * @param {Matrix4} m - A 4x4 matrix of which the upper left 3x3 matrix is a pure rotation matrix.
		 * @return {Vector4} A reference to this vector.
		 */
		setAxisAngleFromRotationMatrix( m ) {

			// http://www.euclideanspace.com/maths/geometry/rotations/conversions/matrixToAngle/index.htm

			// assumes the upper 3x3 of m is a pure rotation matrix (i.e, unscaled)

			let angle, x, y, z; // variables for result
			const epsilon = 0.01,		// margin to allow for rounding errors
				epsilon2 = 0.1,		// margin to distinguish between 0 and 180 degrees

				te = m.elements,

				m11 = te[ 0 ], m12 = te[ 4 ], m13 = te[ 8 ],
				m21 = te[ 1 ], m22 = te[ 5 ], m23 = te[ 9 ],
				m31 = te[ 2 ], m32 = te[ 6 ], m33 = te[ 10 ];

			if ( ( Math.abs( m12 - m21 ) < epsilon ) &&
			     ( Math.abs( m13 - m31 ) < epsilon ) &&
			     ( Math.abs( m23 - m32 ) < epsilon ) ) {

				// singularity found
				// first check for identity matrix which must have +1 for all terms
				// in leading diagonal and zero in other terms

				if ( ( Math.abs( m12 + m21 ) < epsilon2 ) &&
				     ( Math.abs( m13 + m31 ) < epsilon2 ) &&
				     ( Math.abs( m23 + m32 ) < epsilon2 ) &&
				     ( Math.abs( m11 + m22 + m33 - 3 ) < epsilon2 ) ) {

					// this singularity is identity matrix so angle = 0

					this.set( 1, 0, 0, 0 );

					return this; // zero angle, arbitrary axis

				}

				// otherwise this singularity is angle = 180

				angle = Math.PI;

				const xx = ( m11 + 1 ) / 2;
				const yy = ( m22 + 1 ) / 2;
				const zz = ( m33 + 1 ) / 2;
				const xy = ( m12 + m21 ) / 4;
				const xz = ( m13 + m31 ) / 4;
				const yz = ( m23 + m32 ) / 4;

				if ( ( xx > yy ) && ( xx > zz ) ) {

					// m11 is the largest diagonal term

					if ( xx < epsilon ) {

						x = 0;
						y = 0.707106781;
						z = 0.707106781;

					} else {

						x = Math.sqrt( xx );
						y = xy / x;
						z = xz / x;

					}

				} else if ( yy > zz ) {

					// m22 is the largest diagonal term

					if ( yy < epsilon ) {

						x = 0.707106781;
						y = 0;
						z = 0.707106781;

					} else {

						y = Math.sqrt( yy );
						x = xy / y;
						z = yz / y;

					}

				} else {

					// m33 is the largest diagonal term so base result on this

					if ( zz < epsilon ) {

						x = 0.707106781;
						y = 0.707106781;
						z = 0;

					} else {

						z = Math.sqrt( zz );
						x = xz / z;
						y = yz / z;

					}

				}

				this.set( x, y, z, angle );

				return this; // return 180 deg rotation

			}

			// as we have reached here there are no singularities so we can handle normally

			let s = Math.sqrt( ( m32 - m23 ) * ( m32 - m23 ) +
				( m13 - m31 ) * ( m13 - m31 ) +
				( m21 - m12 ) * ( m21 - m12 ) ); // used to normalize

			if ( Math.abs( s ) < 0.001 ) s = 1;

			// prevent divide by zero, should not happen if matrix is orthogonal and should be
			// caught by singularity test above, but I've left it in just in case

			this.x = ( m32 - m23 ) / s;
			this.y = ( m13 - m31 ) / s;
			this.z = ( m21 - m12 ) / s;
			this.w = Math.acos( ( m11 + m22 + m33 - 1 ) / 2 );

			return this;

		}

		/**
		 * Sets the vector components to the position elements of the
		 * given transformation matrix.
		 *
		 * @param {Matrix4} m - The 4x4 matrix.
		 * @return {Vector4} A reference to this vector.
		 */
		setFromMatrixPosition( m ) {

			const e = m.elements;

			this.x = e[ 12 ];
			this.y = e[ 13 ];
			this.z = e[ 14 ];
			this.w = e[ 15 ];

			return this;

		}

		/**
		 * If this vector's x, y, z or w value is greater than the given vector's x, y, z or w
		 * value, replace that value with the corresponding min value.
		 *
		 * @param {Vector4} v - The vector.
		 * @return {Vector4} A reference to this vector.
		 */
		min( v ) {

			this.x = Math.min( this.x, v.x );
			this.y = Math.min( this.y, v.y );
			this.z = Math.min( this.z, v.z );
			this.w = Math.min( this.w, v.w );

			return this;

		}

		/**
		 * If this vector's x, y, z or w value is less than the given vector's x, y, z or w
		 * value, replace that value with the corresponding max value.
		 *
		 * @param {Vector4} v - The vector.
		 * @return {Vector4} A reference to this vector.
		 */
		max( v ) {

			this.x = Math.max( this.x, v.x );
			this.y = Math.max( this.y, v.y );
			this.z = Math.max( this.z, v.z );
			this.w = Math.max( this.w, v.w );

			return this;

		}

		/**
		 * If this vector's x, y, z or w value is greater than the max vector's x, y, z or w
		 * value, it is replaced by the corresponding value.
		 * If this vector's x, y, z or w value is less than the min vector's x, y, z or w value,
		 * it is replaced by the corresponding value.
		 *
		 * @param {Vector4} min - The minimum x, y and z values.
		 * @param {Vector4} max - The maximum x, y and z values in the desired range.
		 * @return {Vector4} A reference to this vector.
		 */
		clamp( min, max ) {

			// assumes min < max, componentwise

			this.x = clamp( this.x, min.x, max.x );
			this.y = clamp( this.y, min.y, max.y );
			this.z = clamp( this.z, min.z, max.z );
			this.w = clamp( this.w, min.w, max.w );

			return this;

		}

		/**
		 * If this vector's x, y, z or w values are greater than the max value, they are
		 * replaced by the max value.
		 * If this vector's x, y, z or w values are less than the min value, they are
		 * replaced by the min value.
		 *
		 * @param {number} minVal - The minimum value the components will be clamped to.
		 * @param {number} maxVal - The maximum value the components will be clamped to.
		 * @return {Vector4} A reference to this vector.
		 */
		clampScalar( minVal, maxVal ) {

			this.x = clamp( this.x, minVal, maxVal );
			this.y = clamp( this.y, minVal, maxVal );
			this.z = clamp( this.z, minVal, maxVal );
			this.w = clamp( this.w, minVal, maxVal );

			return this;

		}

		/**
		 * If this vector's length is greater than the max value, it is replaced by
		 * the max value.
		 * If this vector's length is less than the min value, it is replaced by the
		 * min value.
		 *
		 * @param {number} min - The minimum value the vector length will be clamped to.
		 * @param {number} max - The maximum value the vector length will be clamped to.
		 * @return {Vector4} A reference to this vector.
		 */
		clampLength( min, max ) {

			const length = this.length();

			return this.divideScalar( length || 1 ).multiplyScalar( clamp( length, min, max ) );

		}

		/**
		 * The components of this vector are rounded down to the nearest integer value.
		 *
		 * @return {Vector4} A reference to this vector.
		 */
		floor() {

			this.x = Math.floor( this.x );
			this.y = Math.floor( this.y );
			this.z = Math.floor( this.z );
			this.w = Math.floor( this.w );

			return this;

		}

		/**
		 * The components of this vector are rounded up to the nearest integer value.
		 *
		 * @return {Vector4} A reference to this vector.
		 */
		ceil() {

			this.x = Math.ceil( this.x );
			this.y = Math.ceil( this.y );
			this.z = Math.ceil( this.z );
			this.w = Math.ceil( this.w );

			return this;

		}

		/**
		 * The components of this vector are rounded to the nearest integer value
		 *
		 * @return {Vector4} A reference to this vector.
		 */
		round() {

			this.x = Math.round( this.x );
			this.y = Math.round( this.y );
			this.z = Math.round( this.z );
			this.w = Math.round( this.w );

			return this;

		}

		/**
		 * The components of this vector are rounded towards zero (up if negative,
		 * down if positive) to an integer value.
		 *
		 * @return {Vector4} A reference to this vector.
		 */
		roundToZero() {

			this.x = Math.trunc( this.x );
			this.y = Math.trunc( this.y );
			this.z = Math.trunc( this.z );
			this.w = Math.trunc( this.w );

			return this;

		}

		/**
		 * Inverts this vector - i.e. sets x = -x, y = -y, z = -z, w = -w.
		 *
		 * @return {Vector4} A reference to this vector.
		 */
		negate() {

			this.x = - this.x;
			this.y = - this.y;
			this.z = - this.z;
			this.w = - this.w;

			return this;

		}

		/**
		 * Calculates the dot product of the given vector with this instance.
		 *
		 * @param {Vector4} v - The vector to compute the dot product with.
		 * @return {number} The result of the dot product.
		 */
		dot( v ) {

			return this.x * v.x + this.y * v.y + this.z * v.z + this.w * v.w;

		}

		/**
		 * Computes the square of the Euclidean length (straight-line length) from
		 * (0, 0, 0, 0) to (x, y, z, w). If you are comparing the lengths of vectors, you should
		 * compare the length squared instead as it is slightly more efficient to calculate.
		 *
		 * @return {number} The square length of this vector.
		 */
		lengthSq() {

			return this.x * this.x + this.y * this.y + this.z * this.z + this.w * this.w;

		}

		/**
		 * Computes the  Euclidean length (straight-line length) from (0, 0, 0, 0) to (x, y, z, w).
		 *
		 * @return {number} The length of this vector.
		 */
		length() {

			return Math.sqrt( this.x * this.x + this.y * this.y + this.z * this.z + this.w * this.w );

		}

		/**
		 * Computes the Manhattan length of this vector.
		 *
		 * @return {number} The length of this vector.
		 */
		manhattanLength() {

			return Math.abs( this.x ) + Math.abs( this.y ) + Math.abs( this.z ) + Math.abs( this.w );

		}

		/**
		 * Converts this vector to a unit vector - that is, sets it equal to a vector
		 * with the same direction as this one, but with a vector length of `1`.
		 *
		 * @return {Vector4} A reference to this vector.
		 */
		normalize() {

			return this.divideScalar( this.length() || 1 );

		}

		/**
		 * Sets this vector to a vector with the same direction as this one, but
		 * with the specified length.
		 *
		 * @param {number} length - The new length of this vector.
		 * @return {Vector4} A reference to this vector.
		 */
		setLength( length ) {

			return this.normalize().multiplyScalar( length );

		}

		/**
		 * Linearly interpolates between the given vector and this instance, where
		 * alpha is the percent distance along the line - alpha = 0 will be this
		 * vector, and alpha = 1 will be the given one.
		 *
		 * @param {Vector4} v - The vector to interpolate towards.
		 * @param {number} alpha - The interpolation factor, typically in the closed interval `[0, 1]`.
		 * @return {Vector4} A reference to this vector.
		 */
		lerp( v, alpha ) {

			this.x += ( v.x - this.x ) * alpha;
			this.y += ( v.y - this.y ) * alpha;
			this.z += ( v.z - this.z ) * alpha;
			this.w += ( v.w - this.w ) * alpha;

			return this;

		}

		/**
		 * Linearly interpolates between the given vectors, where alpha is the percent
		 * distance along the line - alpha = 0 will be first vector, and alpha = 1 will
		 * be the second one. The result is stored in this instance.
		 *
		 * @param {Vector4} v1 - The first vector.
		 * @param {Vector4} v2 - The second vector.
		 * @param {number} alpha - The interpolation factor, typically in the closed interval `[0, 1]`.
		 * @return {Vector4} A reference to this vector.
		 */
		lerpVectors( v1, v2, alpha ) {

			this.x = v1.x + ( v2.x - v1.x ) * alpha;
			this.y = v1.y + ( v2.y - v1.y ) * alpha;
			this.z = v1.z + ( v2.z - v1.z ) * alpha;
			this.w = v1.w + ( v2.w - v1.w ) * alpha;

			return this;

		}

		/**
		 * Returns `true` if this vector is equal with the given one.
		 *
		 * @param {Vector4} v - The vector to test for equality.
		 * @return {boolean} Whether this vector is equal with the given one.
		 */
		equals( v ) {

			return ( ( v.x === this.x ) && ( v.y === this.y ) && ( v.z === this.z ) && ( v.w === this.w ) );

		}

		/**
		 * Sets this vector's x value to be `array[ offset ]`, y value to be `array[ offset + 1 ]`,
		 * z value to be `array[ offset + 2 ]`, w value to be `array[ offset + 3 ]`.
		 *
		 * @param {Array<number>} array - An array holding the vector component values.
		 * @param {number} [offset=0] - The offset into the array.
		 * @return {Vector4} A reference to this vector.
		 */
		fromArray( array, offset = 0 ) {

			this.x = array[ offset ];
			this.y = array[ offset + 1 ];
			this.z = array[ offset + 2 ];
			this.w = array[ offset + 3 ];

			return this;

		}

		/**
		 * Writes the components of this vector to the given array. If no array is provided,
		 * the method returns a new instance.
		 *
		 * @param {Array<number>} [array=[]] - The target array holding the vector components.
		 * @param {number} [offset=0] - Index of the first element in the array.
		 * @return {Array<number>} The vector components.
		 */
		toArray( array = [], offset = 0 ) {

			array[ offset ] = this.x;
			array[ offset + 1 ] = this.y;
			array[ offset + 2 ] = this.z;
			array[ offset + 3 ] = this.w;

			return array;

		}

		/**
		 * Sets the components of this vector from the given buffer attribute.
		 *
		 * @param {BufferAttribute} attribute - The buffer attribute holding vector data.
		 * @param {number} index - The index into the attribute.
		 * @return {Vector4} A reference to this vector.
		 */
		fromBufferAttribute( attribute, index ) {

			this.x = attribute.getX( index );
			this.y = attribute.getY( index );
			this.z = attribute.getZ( index );
			this.w = attribute.getW( index );

			return this;

		}

		/**
		 * Sets each component of this vector to a pseudo-random value between `0` and
		 * `1`, excluding `1`.
		 *
		 * @return {Vector4} A reference to this vector.
		 */
		random() {

			this.x = Math.random();
			this.y = Math.random();
			this.z = Math.random();
			this.w = Math.random();

			return this;

		}

		*[ Symbol.iterator ]() {

			yield this.x;
			yield this.y;
			yield this.z;
			yield this.w;

		}

	}

	const _v0 = /*@__PURE__*/ new Vector3();
	const _v1 = /*@__PURE__*/ new Vector3();
	const _v2 = /*@__PURE__*/ new Vector3();
	const _v3 = /*@__PURE__*/ new Vector3();

	const _vab = /*@__PURE__*/ new Vector3();
	const _vac = /*@__PURE__*/ new Vector3();
	const _vbc = /*@__PURE__*/ new Vector3();
	const _vap = /*@__PURE__*/ new Vector3();
	const _vbp = /*@__PURE__*/ new Vector3();
	const _vcp = /*@__PURE__*/ new Vector3();

	const _v40 = /*@__PURE__*/ new Vector4();
	const _v41 = /*@__PURE__*/ new Vector4();
	const _v42 = /*@__PURE__*/ new Vector4();

	/**
	 * A geometric triangle as defined by three vectors representing its three corners.
	 */
	class Triangle {

		/**
		 * Constructs a new triangle.
		 *
		 * @param {Vector3} [a=(0,0,0)] - The first corner of the triangle.
		 * @param {Vector3} [b=(0,0,0)] - The second corner of the triangle.
		 * @param {Vector3} [c=(0,0,0)] - The third corner of the triangle.
		 */
		constructor( a = new Vector3(), b = new Vector3(), c = new Vector3() ) {

			/**
			 * The first corner of the triangle.
			 *
			 * @type {Vector3}
			 */
			this.a = a;

			/**
			 * The second corner of the triangle.
			 *
			 * @type {Vector3}
			 */
			this.b = b;

			/**
			 * The third corner of the triangle.
			 *
			 * @type {Vector3}
			 */
			this.c = c;

		}

		/**
		 * Computes the normal vector of a triangle.
		 *
		 * @param {Vector3} a - The first corner of the triangle.
		 * @param {Vector3} b - The second corner of the triangle.
		 * @param {Vector3} c - The third corner of the triangle.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The triangle's normal.
		 */
		static getNormal( a, b, c, target ) {

			target.subVectors( c, b );
			_v0.subVectors( a, b );
			target.cross( _v0 );

			const targetLengthSq = target.lengthSq();
			if ( targetLengthSq > 0 ) {

				return target.multiplyScalar( 1 / Math.sqrt( targetLengthSq ) );

			}

			return target.set( 0, 0, 0 );

		}

		/**
		 * Computes a barycentric coordinates from the given vector.
		 * Returns `null` if the triangle is degenerate.
		 *
		 * @param {Vector3} point - A point in 3D space.
		 * @param {Vector3} a - The first corner of the triangle.
		 * @param {Vector3} b - The second corner of the triangle.
		 * @param {Vector3} c - The third corner of the triangle.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The barycentric coordinates for the given point
		 */
		static getBarycoord( point, a, b, c, target ) {

			// based on: http://www.blackpawn.com/texts/pointinpoly/default.html

			_v0.subVectors( c, a );
			_v1.subVectors( b, a );
			_v2.subVectors( point, a );

			const dot00 = _v0.dot( _v0 );
			const dot01 = _v0.dot( _v1 );
			const dot02 = _v0.dot( _v2 );
			const dot11 = _v1.dot( _v1 );
			const dot12 = _v1.dot( _v2 );

			const denom = ( dot00 * dot11 - dot01 * dot01 );

			// collinear or singular triangle
			if ( denom === 0 ) {

				target.set( 0, 0, 0 );
				return null;

			}

			const invDenom = 1 / denom;
			const u = ( dot11 * dot02 - dot01 * dot12 ) * invDenom;
			const v = ( dot00 * dot12 - dot01 * dot02 ) * invDenom;

			// barycentric coordinates must always sum to 1
			return target.set( 1 - u - v, v, u );

		}

		/**
		 * Returns `true` if the given point, when projected onto the plane of the
		 * triangle, lies within the triangle.
		 *
		 * @param {Vector3} point - The point in 3D space to test.
		 * @param {Vector3} a - The first corner of the triangle.
		 * @param {Vector3} b - The second corner of the triangle.
		 * @param {Vector3} c - The third corner of the triangle.
		 * @return {boolean} Whether the given point, when projected onto the plane of the
		 * triangle, lies within the triangle or not.
		 */
		static containsPoint( point, a, b, c ) {

			// if the triangle is degenerate then we can't contain a point
			if ( this.getBarycoord( point, a, b, c, _v3 ) === null ) {

				return false;

			}

			return ( _v3.x >= 0 ) && ( _v3.y >= 0 ) && ( ( _v3.x + _v3.y ) <= 1 );

		}

		/**
		 * Computes the value barycentrically interpolated for the given point on the
		 * triangle. Returns `null` if the triangle is degenerate.
		 *
		 * @param {Vector3} point - Position of interpolated point.
		 * @param {Vector3} p1 - The first corner of the triangle.
		 * @param {Vector3} p2 - The second corner of the triangle.
		 * @param {Vector3} p3 - The third corner of the triangle.
		 * @param {Vector3} v1 - Value to interpolate of first vertex.
		 * @param {Vector3} v2 - Value to interpolate of second vertex.
		 * @param {Vector3} v3 - Value to interpolate of third vertex.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The interpolated value.
		 */
		static getInterpolation( point, p1, p2, p3, v1, v2, v3, target ) {

			if ( this.getBarycoord( point, p1, p2, p3, _v3 ) === null ) {

				target.x = 0;
				target.y = 0;
				if ( 'z' in target ) target.z = 0;
				if ( 'w' in target ) target.w = 0;
				return null;

			}

			target.setScalar( 0 );
			target.addScaledVector( v1, _v3.x );
			target.addScaledVector( v2, _v3.y );
			target.addScaledVector( v3, _v3.z );

			return target;

		}

		/**
		 * Computes the value barycentrically interpolated for the given attribute and indices.
		 *
		 * @param {BufferAttribute} attr - The attribute to interpolate.
		 * @param {number} i1 - Index of first vertex.
		 * @param {number} i2 - Index of second vertex.
		 * @param {number} i3 - Index of third vertex.
		 * @param {Vector3} barycoord - The barycoordinate value to use to interpolate.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The interpolated attribute value.
		 */
		static getInterpolatedAttribute( attr, i1, i2, i3, barycoord, target ) {

			_v40.setScalar( 0 );
			_v41.setScalar( 0 );
			_v42.setScalar( 0 );

			_v40.fromBufferAttribute( attr, i1 );
			_v41.fromBufferAttribute( attr, i2 );
			_v42.fromBufferAttribute( attr, i3 );

			target.setScalar( 0 );
			target.addScaledVector( _v40, barycoord.x );
			target.addScaledVector( _v41, barycoord.y );
			target.addScaledVector( _v42, barycoord.z );

			return target;

		}

		/**
		 * Returns `true` if the triangle is oriented towards the given direction.
		 *
		 * @param {Vector3} a - The first corner of the triangle.
		 * @param {Vector3} b - The second corner of the triangle.
		 * @param {Vector3} c - The third corner of the triangle.
		 * @param {Vector3} direction - The (normalized) direction vector.
		 * @return {boolean} Whether the triangle is oriented towards the given direction or not.
		 */
		static isFrontFacing( a, b, c, direction ) {

			_v0.subVectors( c, b );
			_v1.subVectors( a, b );

			// strictly front facing
			return ( _v0.cross( _v1 ).dot( direction ) < 0 ) ? true : false;

		}

		/**
		 * Sets the triangle's vertices by copying the given values.
		 *
		 * @param {Vector3} a - The first corner of the triangle.
		 * @param {Vector3} b - The second corner of the triangle.
		 * @param {Vector3} c - The third corner of the triangle.
		 * @return {Triangle} A reference to this triangle.
		 */
		set( a, b, c ) {

			this.a.copy( a );
			this.b.copy( b );
			this.c.copy( c );

			return this;

		}

		/**
		 * Sets the triangle's vertices by copying the given array values.
		 *
		 * @param {Array<Vector3>} points - An array with 3D points.
		 * @param {number} i0 - The array index representing the first corner of the triangle.
		 * @param {number} i1 - The array index representing the second corner of the triangle.
		 * @param {number} i2 - The array index representing the third corner of the triangle.
		 * @return {Triangle} A reference to this triangle.
		 */
		setFromPointsAndIndices( points, i0, i1, i2 ) {

			this.a.copy( points[ i0 ] );
			this.b.copy( points[ i1 ] );
			this.c.copy( points[ i2 ] );

			return this;

		}

		/**
		 * Sets the triangle's vertices by copying the given attribute values.
		 *
		 * @param {BufferAttribute} attribute - A buffer attribute with 3D points data.
		 * @param {number} i0 - The attribute index representing the first corner of the triangle.
		 * @param {number} i1 - The attribute index representing the second corner of the triangle.
		 * @param {number} i2 - The attribute index representing the third corner of the triangle.
		 * @return {Triangle} A reference to this triangle.
		 */
		setFromAttributeAndIndices( attribute, i0, i1, i2 ) {

			this.a.fromBufferAttribute( attribute, i0 );
			this.b.fromBufferAttribute( attribute, i1 );
			this.c.fromBufferAttribute( attribute, i2 );

			return this;

		}

		/**
		 * Returns a new triangle with copied values from this instance.
		 *
		 * @return {Triangle} A clone of this instance.
		 */
		clone() {

			return new this.constructor().copy( this );

		}

		/**
		 * Copies the values of the given triangle to this instance.
		 *
		 * @param {Triangle} triangle - The triangle to copy.
		 * @return {Triangle} A reference to this triangle.
		 */
		copy( triangle ) {

			this.a.copy( triangle.a );
			this.b.copy( triangle.b );
			this.c.copy( triangle.c );

			return this;

		}

		/**
		 * Computes the area of the triangle.
		 *
		 * @return {number} The triangle's area.
		 */
		getArea() {

			_v0.subVectors( this.c, this.b );
			_v1.subVectors( this.a, this.b );

			return _v0.cross( _v1 ).length() * 0.5;

		}

		/**
		 * Computes the midpoint of the triangle.
		 *
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The triangle's midpoint.
		 */
		getMidpoint( target ) {

			return target.addVectors( this.a, this.b ).add( this.c ).multiplyScalar( 1 / 3 );

		}

		/**
		 * Computes the normal of the triangle.
		 *
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The triangle's normal.
		 */
		getNormal( target ) {

			return Triangle.getNormal( this.a, this.b, this.c, target );

		}

		/**
		 * Computes a plane the triangle lies within.
		 *
		 * @param {Plane} target - The target vector that is used to store the method's result.
		 * @return {Plane} The plane the triangle lies within.
		 */
		getPlane( target ) {

			return target.setFromCoplanarPoints( this.a, this.b, this.c );

		}

		/**
		 * Computes a barycentric coordinates from the given vector.
		 * Returns `null` if the triangle is degenerate.
		 *
		 * @param {Vector3} point - A point in 3D space.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The barycentric coordinates for the given point
		 */
		getBarycoord( point, target ) {

			return Triangle.getBarycoord( point, this.a, this.b, this.c, target );

		}

		/**
		 * Computes the value barycentrically interpolated for the given point on the
		 * triangle. Returns `null` if the triangle is degenerate.
		 *
		 * @param {Vector3} point - Position of interpolated point.
		 * @param {Vector3} v1 - Value to interpolate of first vertex.
		 * @param {Vector3} v2 - Value to interpolate of second vertex.
		 * @param {Vector3} v3 - Value to interpolate of third vertex.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {?Vector3} The interpolated value.
		 */
		getInterpolation( point, v1, v2, v3, target ) {

			return Triangle.getInterpolation( point, this.a, this.b, this.c, v1, v2, v3, target );

		}

		/**
		 * Returns `true` if the given point, when projected onto the plane of the
		 * triangle, lies within the triangle.
		 *
		 * @param {Vector3} point - The point in 3D space to test.
		 * @return {boolean} Whether the given point, when projected onto the plane of the
		 * triangle, lies within the triangle or not.
		 */
		containsPoint( point ) {

			return Triangle.containsPoint( point, this.a, this.b, this.c );

		}

		/**
		 * Returns `true` if the triangle is oriented towards the given direction.
		 *
		 * @param {Vector3} direction - The (normalized) direction vector.
		 * @return {boolean} Whether the triangle is oriented towards the given direction or not.
		 */
		isFrontFacing( direction ) {

			return Triangle.isFrontFacing( this.a, this.b, this.c, direction );

		}

		/**
		 * Returns `true` if this triangle intersects with the given box.
		 *
		 * @param {Box3} box - The box to intersect.
		 * @return {boolean} Whether this triangle intersects with the given box or not.
		 */
		intersectsBox( box ) {

			return box.intersectsTriangle( this );

		}

		/**
		 * Returns the closest point on the triangle to the given point.
		 *
		 * @param {Vector3} p - The point to compute the closest point for.
		 * @param {Vector3} target - The target vector that is used to store the method's result.
		 * @return {Vector3} The closest point on the triangle.
		 */
		closestPointToPoint( p, target ) {

			const a = this.a, b = this.b, c = this.c;
			let v, w;

			// algorithm thanks to Real-Time Collision Detection by Christer Ericson,
			// published by Morgan Kaufmann Publishers, (c) 2005 Elsevier Inc.,
			// under the accompanying license; see chapter 5.1.5 for detailed explanation.
			// basically, we're distinguishing which of the voronoi regions of the triangle
			// the point lies in with the minimum amount of redundant computation.

			_vab.subVectors( b, a );
			_vac.subVectors( c, a );
			_vap.subVectors( p, a );
			const d1 = _vab.dot( _vap );
			const d2 = _vac.dot( _vap );
			if ( d1 <= 0 && d2 <= 0 ) {

				// vertex region of A; barycentric coords (1, 0, 0)
				return target.copy( a );

			}

			_vbp.subVectors( p, b );
			const d3 = _vab.dot( _vbp );
			const d4 = _vac.dot( _vbp );
			if ( d3 >= 0 && d4 <= d3 ) {

				// vertex region of B; barycentric coords (0, 1, 0)
				return target.copy( b );

			}

			const vc = d1 * d4 - d3 * d2;
			if ( vc <= 0 && d1 >= 0 && d3 <= 0 ) {

				v = d1 / ( d1 - d3 );
				// edge region of AB; barycentric coords (1-v, v, 0)
				return target.copy( a ).addScaledVector( _vab, v );

			}

			_vcp.subVectors( p, c );
			const d5 = _vab.dot( _vcp );
			const d6 = _vac.dot( _vcp );
			if ( d6 >= 0 && d5 <= d6 ) {

				// vertex region of C; barycentric coords (0, 0, 1)
				return target.copy( c );

			}

			const vb = d5 * d2 - d1 * d6;
			if ( vb <= 0 && d2 >= 0 && d6 <= 0 ) {

				w = d2 / ( d2 - d6 );
				// edge region of AC; barycentric coords (1-w, 0, w)
				return target.copy( a ).addScaledVector( _vac, w );

			}

			const va = d3 * d6 - d5 * d4;
			if ( va <= 0 && ( d4 - d3 ) >= 0 && ( d5 - d6 ) >= 0 ) {

				_vbc.subVectors( c, b );
				w = ( d4 - d3 ) / ( ( d4 - d3 ) + ( d5 - d6 ) );
				// edge region of BC; barycentric coords (0, 1-w, w)
				return target.copy( b ).addScaledVector( _vbc, w ); // edge region of BC

			}

			// face region
			const denom = 1 / ( va + vb + vc );
			// u = va * denom
			v = vb * denom;
			w = vc * denom;

			return target.copy( a ).addScaledVector( _vab, v ).addScaledVector( _vac, w );

		}

		/**
		 * Returns `true` if this triangle is equal with the given one.
		 *
		 * @param {Triangle} triangle - The triangle to test for equality.
		 * @return {boolean} Whether this triangle is equal with the given one.
		 */
		equals( triangle ) {

			return triangle.a.equals( this.a ) && triangle.b.equals( this.b ) && triangle.c.equals( this.c );

		}

	}

	const _inverseMatrix = /*@__PURE__*/ new Matrix4();
	const _ray = /*@__PURE__*/ new Ray();
	const _sphere = /*@__PURE__*/ new Sphere();
	const _sphereHitAt = /*@__PURE__*/ new Vector3();

	const _vA = /*@__PURE__*/ new Vector3();
	const _vB = /*@__PURE__*/ new Vector3();
	const _vC = /*@__PURE__*/ new Vector3();

	const _tempA = /*@__PURE__*/ new Vector3();
	const _morphA = /*@__PURE__*/ new Vector3();

	const _intersectionPoint = /*@__PURE__*/ new Vector3();
	const _intersectionPointWorld = /*@__PURE__*/ new Vector3();

	/**
	 * Class representing triangular polygon mesh based objects.
	 *
	 * ```js
	 * const geometry = new THREE.BoxGeometry( 1, 1, 1 );
	 * const material = new THREE.MeshBasicMaterial( { color: 0xffff00 } );
	 * const mesh = new THREE.Mesh( geometry, material );
	 * scene.add( mesh );
	 * ```
	 *
	 * @augments Object3D
	 */
	class Mesh extends Object3D {

		/**
		 * Constructs a new mesh.
		 *
		 * @param {BufferGeometry} [geometry] - The mesh geometry.
		 * @param {Material|Array<Material>} [material] - The mesh material.
		 */
		constructor( geometry = new BufferGeometry(), material = new MeshBasicMaterial() ) {

			super();

			/**
			 * This flag can be used for type testing.
			 *
			 * @type {boolean}
			 * @readonly
			 * @default true
			 */
			this.isMesh = true;

			this.type = 'Mesh';

			/**
			 * The mesh geometry.
			 *
			 * @type {BufferGeometry}
			 */
			this.geometry = geometry;

			/**
			 * The mesh material.
			 *
			 * @type {Material|Array<Material>}
			 * @default MeshBasicMaterial
			 */
			this.material = material;

			/**
			 * A dictionary representing the morph targets in the geometry. The key is the
			 * morph targets name, the value its attribute index. This member is `undefined`
			 * by default and only set when morph targets are detected in the geometry.
			 *
			 * @type {Object<String,number>|undefined}
			 * @default undefined
			 */
			this.morphTargetDictionary = undefined;

			/**
			 * An array of weights typically in the range `[0,1]` that specify how much of the morph
			 * is applied. This member is `undefined` by default and only set when morph targets are
			 * detected in the geometry.
			 *
			 * @type {Array<number>|undefined}
			 * @default undefined
			 */
			this.morphTargetInfluences = undefined;

			/**
			 * The number of instances of this mesh.
			 * Can only be used with {@link WebGPURenderer}.
			 *
			 * @type {number}
			 * @default 1
			 */
			this.count = 1;

			this.updateMorphTargets();

		}

		copy( source, recursive ) {

			super.copy( source, recursive );

			if ( source.morphTargetInfluences !== undefined ) {

				this.morphTargetInfluences = source.morphTargetInfluences.slice();

			}

			if ( source.morphTargetDictionary !== undefined ) {

				this.morphTargetDictionary = Object.assign( {}, source.morphTargetDictionary );

			}

			this.material = Array.isArray( source.material ) ? source.material.slice() : source.material;
			this.geometry = source.geometry;

			return this;

		}

		/**
		 * Sets the values of {@link Mesh#morphTargetDictionary} and {@link Mesh#morphTargetInfluences}
		 * to make sure existing morph targets can influence this 3D object.
		 */
		updateMorphTargets() {

			const geometry = this.geometry;

			const morphAttributes = geometry.morphAttributes;
			const keys = Object.keys( morphAttributes );

			if ( keys.length > 0 ) {

				const morphAttribute = morphAttributes[ keys[ 0 ] ];

				if ( morphAttribute !== undefined ) {

					this.morphTargetInfluences = [];
					this.morphTargetDictionary = {};

					for ( let m = 0, ml = morphAttribute.length; m < ml; m ++ ) {

						const name = morphAttribute[ m ].name || String( m );

						this.morphTargetInfluences.push( 0 );
						this.morphTargetDictionary[ name ] = m;

					}

				}

			}

		}

		/**
		 * Returns the local-space position of the vertex at the given index, taking into
		 * account the current animation state of both morph targets and skinning.
		 *
		 * @param {number} index - The vertex index.
		 * @param {Vector3} target - The target object that is used to store the method's result.
		 * @return {Vector3} The vertex position in local space.
		 */
		getVertexPosition( index, target ) {

			const geometry = this.geometry;
			const position = geometry.attributes.position;
			const morphPosition = geometry.morphAttributes.position;
			const morphTargetsRelative = geometry.morphTargetsRelative;

			target.fromBufferAttribute( position, index );

			const morphInfluences = this.morphTargetInfluences;

			if ( morphPosition && morphInfluences ) {

				_morphA.set( 0, 0, 0 );

				for ( let i = 0, il = morphPosition.length; i < il; i ++ ) {

					const influence = morphInfluences[ i ];
					const morphAttribute = morphPosition[ i ];

					if ( influence === 0 ) continue;

					_tempA.fromBufferAttribute( morphAttribute, index );

					if ( morphTargetsRelative ) {

						_morphA.addScaledVector( _tempA, influence );

					} else {

						_morphA.addScaledVector( _tempA.sub( target ), influence );

					}

				}

				target.add( _morphA );

			}

			return target;

		}

		/**
		 * Computes intersection points between a casted ray and this line.
		 *
		 * @param {Raycaster} raycaster - The raycaster.
		 * @param {Array<Object>} intersects - The target array that holds the intersection points.
		 */
		raycast( raycaster, intersects ) {

			const geometry = this.geometry;
			const material = this.material;
			const matrixWorld = this.matrixWorld;

			if ( material === undefined ) return;

			// test with bounding sphere in world space

			if ( geometry.boundingSphere === null ) geometry.computeBoundingSphere();

			_sphere.copy( geometry.boundingSphere );
			_sphere.applyMatrix4( matrixWorld );

			// check distance from ray origin to bounding sphere

			_ray.copy( raycaster.ray ).recast( raycaster.near );

			if ( _sphere.containsPoint( _ray.origin ) === false ) {

				if ( _ray.intersectSphere( _sphere, _sphereHitAt ) === null ) return;

				if ( _ray.origin.distanceToSquared( _sphereHitAt ) > ( raycaster.far - raycaster.near ) ** 2 ) return;

			}

			// convert ray to local space of mesh

			_inverseMatrix.copy( matrixWorld ).invert();
			_ray.copy( raycaster.ray ).applyMatrix4( _inverseMatrix );

			// test with bounding box in local space

			if ( geometry.boundingBox !== null ) {

				if ( _ray.intersectsBox( geometry.boundingBox ) === false ) return;

			}

			// test for intersections with geometry

			this._computeIntersections( raycaster, intersects, _ray );

		}

		_computeIntersections( raycaster, intersects, rayLocalSpace ) {

			let intersection;

			const geometry = this.geometry;
			const material = this.material;

			const index = geometry.index;
			const position = geometry.attributes.position;
			const uv = geometry.attributes.uv;
			const uv1 = geometry.attributes.uv1;
			const normal = geometry.attributes.normal;
			const groups = geometry.groups;
			const drawRange = geometry.drawRange;

			if ( index !== null ) {

				// indexed buffer geometry

				if ( Array.isArray( material ) ) {

					for ( let i = 0, il = groups.length; i < il; i ++ ) {

						const group = groups[ i ];
						const groupMaterial = material[ group.materialIndex ];

						const start = Math.max( group.start, drawRange.start );
						const end = Math.min( index.count, Math.min( ( group.start + group.count ), ( drawRange.start + drawRange.count ) ) );

						for ( let j = start, jl = end; j < jl; j += 3 ) {

							const a = index.getX( j );
							const b = index.getX( j + 1 );
							const c = index.getX( j + 2 );

							intersection = checkGeometryIntersection( this, groupMaterial, raycaster, rayLocalSpace, uv, uv1, normal, a, b, c );

							if ( intersection ) {

								intersection.faceIndex = Math.floor( j / 3 ); // triangle number in indexed buffer semantics
								intersection.face.materialIndex = group.materialIndex;
								intersects.push( intersection );

							}

						}

					}

				} else {

					const start = Math.max( 0, drawRange.start );
					const end = Math.min( index.count, ( drawRange.start + drawRange.count ) );

					for ( let i = start, il = end; i < il; i += 3 ) {

						const a = index.getX( i );
						const b = index.getX( i + 1 );
						const c = index.getX( i + 2 );

						intersection = checkGeometryIntersection( this, material, raycaster, rayLocalSpace, uv, uv1, normal, a, b, c );

						if ( intersection ) {

							intersection.faceIndex = Math.floor( i / 3 ); // triangle number in indexed buffer semantics
							intersects.push( intersection );

						}

					}

				}

			} else if ( position !== undefined ) {

				// non-indexed buffer geometry

				if ( Array.isArray( material ) ) {

					for ( let i = 0, il = groups.length; i < il; i ++ ) {

						const group = groups[ i ];
						const groupMaterial = material[ group.materialIndex ];

						const start = Math.max( group.start, drawRange.start );
						const end = Math.min( position.count, Math.min( ( group.start + group.count ), ( drawRange.start + drawRange.count ) ) );

						for ( let j = start, jl = end; j < jl; j += 3 ) {

							const a = j;
							const b = j + 1;
							const c = j + 2;

							intersection = checkGeometryIntersection( this, groupMaterial, raycaster, rayLocalSpace, uv, uv1, normal, a, b, c );

							if ( intersection ) {

								intersection.faceIndex = Math.floor( j / 3 ); // triangle number in non-indexed buffer semantics
								intersection.face.materialIndex = group.materialIndex;
								intersects.push( intersection );

							}

						}

					}

				} else {

					const start = Math.max( 0, drawRange.start );
					const end = Math.min( position.count, ( drawRange.start + drawRange.count ) );

					for ( let i = start, il = end; i < il; i += 3 ) {

						const a = i;
						const b = i + 1;
						const c = i + 2;

						intersection = checkGeometryIntersection( this, material, raycaster, rayLocalSpace, uv, uv1, normal, a, b, c );

						if ( intersection ) {

							intersection.faceIndex = Math.floor( i / 3 ); // triangle number in non-indexed buffer semantics
							intersects.push( intersection );

						}

					}

				}

			}

		}

	}

	function checkIntersection( object, material, raycaster, ray, pA, pB, pC, point ) {

		let intersect;

		if ( material.side === BackSide ) {

			intersect = ray.intersectTriangle( pC, pB, pA, true, point );

		} else {

			intersect = ray.intersectTriangle( pA, pB, pC, ( material.side === FrontSide ), point );

		}

		if ( intersect === null ) return null;

		_intersectionPointWorld.copy( point );
		_intersectionPointWorld.applyMatrix4( object.matrixWorld );

		const distance = raycaster.ray.origin.distanceTo( _intersectionPointWorld );

		if ( distance < raycaster.near || distance > raycaster.far ) return null;

		return {
			distance: distance,
			point: _intersectionPointWorld.clone(),
			object: object
		};

	}

	function checkGeometryIntersection( object, material, raycaster, ray, uv, uv1, normal, a, b, c ) {

		object.getVertexPosition( a, _vA );
		object.getVertexPosition( b, _vB );
		object.getVertexPosition( c, _vC );

		const intersection = checkIntersection( object, material, raycaster, ray, _vA, _vB, _vC, _intersectionPoint );

		if ( intersection ) {

			const barycoord = new Vector3();
			Triangle.getBarycoord( _intersectionPoint, _vA, _vB, _vC, barycoord );

			if ( uv ) {

				intersection.uv = Triangle.getInterpolatedAttribute( uv, a, b, c, barycoord, new Vector2() );

			}

			if ( uv1 ) {

				intersection.uv1 = Triangle.getInterpolatedAttribute( uv1, a, b, c, barycoord, new Vector2() );

			}

			if ( normal ) {

				intersection.normal = Triangle.getInterpolatedAttribute( normal, a, b, c, barycoord, new Vector3() );

				if ( intersection.normal.dot( ray.direction ) > 0 ) {

					intersection.normal.multiplyScalar( - 1 );

				}

			}

			const face = {
				a: a,
				b: b,
				c: c,
				normal: new Vector3(),
				materialIndex: 0
			};

			Triangle.getNormal( _vA, _vB, _vC, face.normal );

			intersection.face = face;
			intersection.barycoord = barycoord;

		}

		return intersection;

	}

	function box(size, color) {
	  const mesh = new Mesh(new BoxGeometry(size, size, size), new MeshBasicMaterial({ color }));
	  mesh.name = "box" + size;
	  return mesh;
	}

	function ball(radius) {
	  const mesh = new Mesh(new SphereGeometry(radius, 12, 8), new MeshBasicMaterial({ color: 0x3366ff }));
	  mesh.name = "ball" + radius;
	  return mesh;
	}

	function buildWorld() {
	  const scene = new Scene();
	  const group = new Group();
	  group.add(box(1, 0xff0000));
	  group.add(ball(0.5));
	  group.children[1].position.set(3, 0, 0);
	  group.rotation.y = Math.PI / 4;
	  scene.add(group);
	  const camera = new PerspectiveCamera(60, 16 / 9, 0.1, 100);
	  camera.position.set(0, 2, 8);
	  camera.lookAt(0, 0, 0);
	  camera.updateMatrixWorld(true);
	  scene.updateMatrixWorld(true);
	  return { scene, group, camera };
	}

	const _matrix = /*@__PURE__*/ new Matrix4();

	/**
	 * This class is designed to assist with raycasting. Raycasting is used for
	 * mouse picking (working out what objects in the 3d space the mouse is over)
	 * amongst other things.
	 */
	class Raycaster {

		/**
		 * Constructs a new raycaster.
		 *
		 * @param {Vector3} origin - The origin vector where the ray casts from.
		 * @param {Vector3} direction - The (normalized) direction vector that gives direction to the ray.
		 * @param {number} [near=0] - All results returned are further away than near. Near can't be negative.
		 * @param {number} [far=Infinity] - All results returned are closer than far. Far can't be lower than near.
		 */
		constructor( origin, direction, near = 0, far = Infinity ) {

			/**
			 * The ray used for raycasting.
			 *
			 * @type {Ray}
			 */
			this.ray = new Ray( origin, direction );

			/**
			 * All results returned are further away than near. Near can't be negative.
			 *
			 * @type {number}
			 * @default 0
			 */
			this.near = near;

			/**
			 * All results returned are further away than near. Near can't be negative.
			 *
			 * @type {number}
			 * @default Infinity
			 */
			this.far = far;

			/**
			 * The camera to use when raycasting against view-dependent objects such as
			 * billboarded objects like sprites. This field can be set manually or
			 * is set when calling `setFromCamera()`.
			 *
			 * @type {?Camera}
			 * @default null
			 */
			this.camera = null;

			/**
			 * Allows to selectively ignore 3D objects when performing intersection tests.
			 * The following code example ensures that only 3D objects on layer `1` will be
			 * honored by raycaster.
			 * ```js
			 * raycaster.layers.set( 1 );
			 * object.layers.enable( 1 );
			 * ```
			 *
			 * @type {Layers}
			 */
			this.layers = new Layers();


			/**
			 * A parameter object that configures the raycasting. It has the structure:
			 *
			 * ```
			 * {
			 * 	Mesh: {},
			 * 	Line: { threshold: 1 },
			 * 	LOD: {},
			 * 	Points: { threshold: 1 },
			 * 	Sprite: {}
			 * }
			 * ```
			 * Where `threshold` is the precision of the raycaster when intersecting objects, in world units.
			 *
			 * @type {Object}
			 */
			this.params = {
				Mesh: {},
				Line: { threshold: 1 },
				LOD: {},
				Points: { threshold: 1 },
				Sprite: {}
			};

		}

		/**
		 * Updates the ray with a new origin and direction by copying the values from the arguments.
		 *
		 * @param {Vector3} origin - The origin vector where the ray casts from.
		 * @param {Vector3} direction - The (normalized) direction vector that gives direction to the ray.
		 */
		set( origin, direction ) {

			// direction is assumed to be normalized (for accurate distance calculations)

			this.ray.set( origin, direction );

		}

		/**
		 * Uses the given coordinates and camera to compute a new origin and direction for the internal ray.
		 *
		 * @param {Vector2} coords - 2D coordinates of the mouse, in normalized device coordinates (NDC).
		 * X and Y components should be between `-1` and `1`.
		 * @param {Camera} camera - The camera from which the ray should originate.
		 */
		setFromCamera( coords, camera ) {

			if ( camera.isPerspectiveCamera ) {

				this.ray.origin.setFromMatrixPosition( camera.matrixWorld );
				this.ray.direction.set( coords.x, coords.y, 0.5 ).unproject( camera ).sub( this.ray.origin ).normalize();
				this.camera = camera;

			} else if ( camera.isOrthographicCamera ) {

				this.ray.origin.set( coords.x, coords.y, ( camera.near + camera.far ) / ( camera.near - camera.far ) ).unproject( camera ); // set origin in plane of camera
				this.ray.direction.set( 0, 0, - 1 ).transformDirection( camera.matrixWorld );
				this.camera = camera;

			} else {

				console.error( 'THREE.Raycaster: Unsupported camera type: ' + camera.type );

			}

		}

		/**
		 * Uses the given WebXR controller to compute a new origin and direction for the internal ray.
		 *
		 * @param {WebXRController} controller - The controller to copy the position and direction from.
		 * @return {Raycaster} A reference to this raycaster.
		 */
		setFromXRController( controller ) {

			_matrix.identity().extractRotation( controller.matrixWorld );

			this.ray.origin.setFromMatrixPosition( controller.matrixWorld );
			this.ray.direction.set( 0, 0, - 1 ).applyMatrix4( _matrix );

			return this;

		}

		/**
		 * The intersection point of a raycaster intersection test.
		 * @typedef {Object} Raycaster~Intersection
		 * @property {number} distance - The distance from the ray's origin to the intersection point.
		 * @property {number} distanceToRay -  Some 3D objects e.g. {@link Points} provide the distance of the
		 * intersection to the nearest point on the ray. For other objects it will be `undefined`.
		 * @property {Vector3} point - The intersection point, in world coordinates.
		 * @property {Object} face - The face that has been intersected.
		 * @property {number} faceIndex - The face index.
		 * @property {Object3D} object - The 3D object that has been intersected.
		 * @property {Vector2} uv - U,V coordinates at point of intersection.
		 * @property {Vector2} uv1 - Second set of U,V coordinates at point of intersection.
		 * @property {Vector3} uv1 - Interpolated normal vector at point of intersection.
		 * @property {number} instanceId - The index number of the instance where the ray
		 * intersects the {@link InstancedMesh}.
		 */

		/**
		 * Checks all intersection between the ray and the object with or without the
		 * descendants. Intersections are returned sorted by distance, closest first.
		 *
		 * `Raycaster` delegates to the `raycast()` method of the passed 3D object, when
		 * evaluating whether the ray intersects the object or not. This allows meshes to respond
		 * differently to ray casting than lines or points.
		 *
		 * Note that for meshes, faces must be pointed towards the origin of the ray in order
		 * to be detected; intersections of the ray passing through the back of a face will not
		 * be detected. To raycast against both faces of an object, you'll want to set  {@link Material#side}
		 * to `THREE.DoubleSide`.
		 *
		 * @param {Object3D} object - The 3D object to check for intersection with the ray.
		 * @param {boolean} [recursive=true] - If set to `true`, it also checks all descendants.
		 * Otherwise it only checks intersection with the object.
		 * @param {Array<Raycaster~Intersection>} [intersects=[]] The target array that holds the result of the method.
		 * @return {Array<Raycaster~Intersection>} An array holding the intersection points.
		 */
		intersectObject( object, recursive = true, intersects = [] ) {

			intersect( object, this, intersects, recursive );

			intersects.sort( ascSort );

			return intersects;

		}

		/**
		 * Checks all intersection between the ray and the objects with or without
		 * the descendants. Intersections are returned sorted by distance, closest first.
		 *
		 * @param {Array<Object3D>} objects - The 3D objects to check for intersection with the ray.
		 * @param {boolean} [recursive=true] - If set to `true`, it also checks all descendants.
		 * Otherwise it only checks intersection with the object.
		 * @param {Array<Raycaster~Intersection>} [intersects=[]] The target array that holds the result of the method.
		 * @return {Array<Raycaster~Intersection>} An array holding the intersection points.
		 */
		intersectObjects( objects, recursive = true, intersects = [] ) {

			for ( let i = 0, l = objects.length; i < l; i ++ ) {

				intersect( objects[ i ], this, intersects, recursive );

			}

			intersects.sort( ascSort );

			return intersects;

		}

	}

	function ascSort( a, b ) {

		return a.distance - b.distance;

	}

	function intersect( object, raycaster, intersects, recursive ) {

		let propagate = true;

		if ( object.layers.test( raycaster.layers ) ) {

			const result = object.raycast( raycaster, intersects );

			if ( result === false ) propagate = false;

		}

		if ( propagate === true && recursive === true ) {

			const children = object.children;

			for ( let i = 0, l = children.length; i < l; i ++ ) {

				intersect( children[ i ], raycaster, intersects, true );

			}

		}

	}

	function pick(world, x, y) {
	  const raycaster = new Raycaster();
	  raycaster.setFromCamera(new Vector2(x, y), world.camera);
	  return raycaster.intersectObjects(world.scene.children, true).map((hit) => hit.object.name + "@" + hit.distance.toFixed(3));
	}

	const fixed = (v) => [v.x, v.y, v.z].map((n) => n.toFixed(3)).join(",");
	const q = new Quaternion().setFromAxisAngle(new Vector3(0, 1, 0), MathUtils.degToRad(90));
	const v = new Vector3(1, 2, 3).applyQuaternion(q);
	console.log("rotated", fixed(v));
	const m = new Matrix4().makeTranslation(1, 2, 3).multiply(new Matrix4().makeScale(2, 2, 2));
	console.log("transformed", fixed(new Vector3(1, 1, 1).applyMatrix4(m)));
	const world = buildWorld();
	const bounds = new Box3().setFromObject(world.scene);
	console.log("bounds", fixed(bounds.min), fixed(bounds.max));
	console.log("picked", pick(world, 0, 0).join(" "));
	console.log("color", new Color(0x336699).getHexString(), new Color("hsl(120, 50%, 50%)").getHexString());
	console.log("children", world.group.children.map((child) => child.name).join(","));

})();
//# sourceMappingURL=main.js.map
})();
