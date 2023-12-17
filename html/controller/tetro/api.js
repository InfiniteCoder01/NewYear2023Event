const processMessage = (msg, callback) => {
    msg.data.arrayBuffer().then(buffer => {
        const data = new DataView(buffer);

        const [width, height] = [data.getUint32(0, true), data.getUint32(4, true)];
        let cursor = 8;

        const field = Array.from({ length: height },
            (_, y) => Array.from({ length: width },
                (_, x) => data.getUint32(cursor + (x + y * width) * 4, true)
            )
        );
        cursor += width * height * 4;

        const zoneMeter = data.getFloat64(cursor, true);
        const zoneMax = data.getFloat64(cursor + 8, true);
        const zoneLinesCount = data.getUint32(cursor + 16, true);
        cursor += 20;

        const zoneLines = Array.from({ length: zoneLinesCount },
            (_, index) => data.getFloat64(cursor + index * 8, true)
        );
        cursor += zoneLinesCount * 8;

        let tetromino = {
            x: data.getInt32(cursor, true),
            y: data.getInt32(cursor + 4, true),
            size: data.getUint32(cursor + 8, true),
            color: data.getUint32(cursor + 12, true),
        };

        let blockCount = data.getUint32(cursor + 16, true);
        cursor += 20;

        tetromino.blocks = Array.from({ length: blockCount }, () => {
            const [x, y] = [data.getUint8(cursor), data.getUint8(cursor + 1)];
            cursor += 2;
            return { x, y };
        });

        callback({
            width, height, field,
            zoneMeter, zoneMax, zoneLines,
            tetromino,

            get: function ({ x, y }) {
                if (x < 0 || x >= this.width || y < 0 || y >= this.height) {
                    return 0;
                }
                return this.field[y][x];
            },

            set: function ({ x, y }, value) {
                if (x < 0 || x >= this.width || y < 0 || y >= this.height) {
                    return;
                }
                this.field[y][x] = value;
            },

            try_move: function (direction) {
                this.tetromino.x += direction;
                if (!this.fits()) {
                    this.tetromino.x += direction;
                    return false;
                }
                return true;
            },

            try_turn: function (ccw) {
                this.turn(ccw);
                if (!this.fits()) {
                    this.turn(!ccw)
                    return false;
                }
                return true;
            },

            turn: function (ccw) {
                for (let i = 0; i < this.tetromino.blocks.length; i++) {
                    let block = this.tetromino.blocks[i];
                    let last = { ...block };
                    if (ccw) {
                        this.tetromino.blocks[i].x = last.y;
                        this.tetromino.blocks[i].y = this.tetromino.size - last.x - 1;
                    } else {
                        this.tetromino.blocks[i].x = this.tetromino.size - last.y - 1;
                        this.tetromino.blocks[i].y = last.x;
                    }
                }
            },

            drop: function () {
                while (true) {
                    this.tetromino.y++;
                    if (!this.fits()) {
                        this.tetromino.y--;
                        return;
                    }
                }
            },

            place: function () {
                for (const block of this.blocks()) {
                    this.set(block, this.tetromino.color);
                }
            },

            unplace: function () {
                for (const block of this.blocks()) {
                    this.set(block, 0);
                }
            },

            fits: function () {
                for (const block of this.blocks()) {
                    if (block.x < 0
                        || block.x >= width
                        || block.y < 0
                        || block.y >= height - zoneLines.length
                        || this.get(block) != 0) {
                        return false;
                    }
                }
                return true;
            },

            blocks: function () {
                return this.tetromino.blocks.map(block => {
                    return {
                        x: this.tetromino.x + block.x,
                        y: this.tetromino.y + block.y
                    };
                });
            }
        });
    });
};

export default processMessage;