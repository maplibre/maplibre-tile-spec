//Ts port of https://github.com/mapbox/point-geometry/blob/main/index.js

export default class Point {
    /**
     * A standalone point geometry with useful accessor, comparison, and
     * modification methods.
     *
     * @class
     * @param {number} x the x-coordinate. This could be longitude or screen pixels, or any other sort of unit.
     * @param {number} y the y-coordinate. This could be latitude or screen pixels, or any other sort of unit.
     *
     * @example
     * const point = new Point(-77, 38);
     */
    constructor(
        public x,
        public y,
    ) {}

    /**
     * Clone this point, returning a new point that can be modified
     * without affecting the old one.
     * @return {Point} the clone
     */
    clone() {
        return new Point(this.x, this.y);
    }

    /**
     * Add this point's x & y coordinates to another point,
     * yielding a new point.
     * @param {Point} p the other point
     * @return {Point} output point
     */
    add(p) {
        return this.clone()._add(p);
    }

    /**
     * Subtract this point's x & y coordinates to from point,
     * yielding a new point.
     * @param {Point} p the other point
     * @return {Point} output point
     */
    sub(p) {
        return this.clone()._sub(p);
    }

    /**
     * Multiply this point's x & y coordinates by point,
     * yielding a new point.
     * @param {Point} p the other point
     * @return {Point} output point
     */
    multByPoint(p) {
        return this.clone()._multByPoint(p);
    }

    /**
     * Divide this point's x & y coordinates by point,
     * yielding a new point.
     * @param {Point} p the other point
     * @return {Point} output point
     */
    divByPoint(p) {
        return this.clone()._divByPoint(p);
    }

    /**
     * Multiply this point's x & y coordinates by a factor,
     * yielding a new point.
     * @param {number} k factor
     * @return {Point} output point
     */
    mult(k) {
        return this.clone()._mult(k);
    }

    /**
     * Divide this point's x & y coordinates by a factor,
     * yielding a new point.
     * @param {number} k factor
     * @return {Point} output point
     */
    div(k) {
        return this.clone()._div(k);
    }

    /**
     * Rotate this point around the 0, 0 origin by an angle a,
     * given in radians
     * @param {number} a angle to rotate around, in radians
     * @return {Point} output point
     */
    rotate(a) {
        return this.clone()._rotate(a);
    }

    /**
     * Rotate this point around p point by an angle a,
     * given in radians
     * @param {number} a angle to rotate around, in radians
     * @param {Point} p Point to rotate around
     * @return {Point} output point
     */
    rotateAround(a, p) {
        return this.clone()._rotateAround(a, p);
    }

    /**
     * Multiply this point by a 4x1 transformation matrix
     * @param {[number, number, number, number]} m transformation matrix
     * @return {Point} output point
     */
    matMult(m) {
        return this.clone()._matMult(m);
    }

    /**
     * Calculate this point but as a unit vector from 0, 0, meaning
     * that the distance from the resulting point to the 0, 0
     * coordinate will be equal to 1 and the angle from the resulting
     * point to the 0, 0 coordinate will be the same as before.
     * @return {Point} unit vector point
     */
    unit() {
        return this.clone()._unit();
    }

    /**
     * Compute a perpendicular point, where the new y coordinate
     * is the old x coordinate and the new x coordinate is the old y
     * coordinate multiplied by -1
     * @return {Point} perpendicular point
     */
    perp() {
        return this.clone()._perp();
    }

    /**
     * Return a version of this point with the x & y coordinates
     * rounded to integers.
     * @return {Point} rounded point
     */
    round() {
        return this.clone()._round();
    }

    /**
     * Return the magnitude of this point: this is the Euclidean
     * distance from the 0, 0 coordinate to this point's x and y
     * coordinates.
     * @return {number} magnitude
     */
    mag() {
        return Math.sqrt(this.x * this.x + this.y * this.y);
    }

    /**
     * Judge whether this point is equal to another point, returning
     * true or false.
     * @param {Point} other the other point
     * @return {boolean} whether the points are equal
     */
    equals(other) {
        return this.x === other.x && this.y === other.y;
    }

    /**
     * Calculate the distance from this point to another point
     * @param {Point} p the other point
     * @return {number} distance
     */
    dist(p) {
        return Math.sqrt(this.distSqr(p));
    }

    /**
     * Calculate the distance from this point to another point,
     * without the square root step. Useful if you're comparing
     * relative distances.
     * @param {Point} p the other point
     * @return {number} distance
     */
    distSqr(p) {
        const dx = p.x - this.x,
            dy = p.y - this.y;
        return dx * dx + dy * dy;
    }

    /**
     * Get the angle from the 0, 0 coordinate to this point, in radians
     * coordinates.
     * @return {number} angle
     */
    angle() {
        return Math.atan2(this.y, this.x);
    }

    /**
     * Get the angle from this point to another point, in radians
     * @param {Point} b the other point
     * @return {number} angle
     */
    angleTo(b) {
        return Math.atan2(this.y - b.y, this.x - b.x);
    }

    /**
     * Get the angle between this point and another point, in radians
     * @param {Point} b the other point
     * @return {number} angle
     */
    angleWith(b) {
        return this.angleWithSep(b.x, b.y);
    }

    /**
     * Find the angle of the two vectors, solving the formula for
     * the cross product a x b = |a||b|sin(θ) for θ.
     * @param {number} x the x-coordinate
     * @param {number} y the y-coordinate
     * @return {number} the angle in radians
     */
    angleWithSep(x, y) {
        return Math.atan2(this.x * y - this.y * x, this.x * x + this.y * y);
    }

    /** @param {[number, number, number, number]} m */
    _matMult(m) {
        const x = m[0] * this.x + m[1] * this.y,
            y = m[2] * this.x + m[3] * this.y;
        this.x = x;
        this.y = y;
        return this;
    }

    /** @param {Point} p */
    _add(p) {
        this.x += p.x;
        this.y += p.y;
        return this;
    }

    /** @param {Point} p */
    _sub(p) {
        this.x -= p.x;
        this.y -= p.y;
        return this;
    }

    /** @param {number} k */
    _mult(k) {
        this.x *= k;
        this.y *= k;
        return this;
    }

    /** @param {number} k */
    _div(k) {
        this.x /= k;
        this.y /= k;
        return this;
    }

    /** @param {Point} p */
    _multByPoint(p) {
        this.x *= p.x;
        this.y *= p.y;
        return this;
    }

    /** @param {Point} p */
    _divByPoint(p) {
        this.x /= p.x;
        this.y /= p.y;
        return this;
    }

    _unit() {
        this._div(this.mag());
        return this;
    }

    _perp() {
        const y = this.y;
        this.y = this.x;
        this.x = -y;
        return this;
    }

    /** @param {number} angle */
    _rotate(angle) {
        const cos = Math.cos(angle),
            sin = Math.sin(angle),
            x = cos * this.x - sin * this.y,
            y = sin * this.x + cos * this.y;
        this.x = x;
        this.y = y;
        return this;
    }

    /**
     * @param {number} angle
     * @param {Point} p
     */
    _rotateAround(angle, p) {
        const cos = Math.cos(angle),
            sin = Math.sin(angle),
            x = p.x + cos * (this.x - p.x) - sin * (this.y - p.y),
            y = p.y + sin * (this.x - p.x) + cos * (this.y - p.y);
        this.x = x;
        this.y = y;
        return this;
    }

    _round() {
        this.x = Math.round(this.x);
        this.y = Math.round(this.y);
        return this;
    }
}

/**
 * Construct a point from an array if necessary, otherwise if the input
 * is already a Point, return it unchanged.
 * @param {Point | [number, number] | {x: number, y: number}} p input value
 * @return {Point} constructed point.
 * @example
 * // this
 * var point = Point.convert([0, 1]);
 * // is equivalent to
 * var point = new Point(0, 1);
 */
/*
Point.convert = function (p) {
    if (p instanceof Point) {
        return /!** @type {Point} *!/ (p);
    }
    if (Array.isArray(p)) {
        return new Point(+p[0], +p[1]);
    }
    if (p.x !== undefined && p.y !== undefined) {
        return new Point(+p.x, +p.y);
    }
    throw new Error('Expected [x, y] or {x, y} point format');
};*/
