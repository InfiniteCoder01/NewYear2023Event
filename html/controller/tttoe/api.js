const apiName = "tttoe";

const processMessage = (msg, callback) => {
    msg.data.arrayBuffer().then(buffer => {
        const data = new DataView(buffer);

        const [width, height] = [data.getUint32(0, true), data.getUint32(4, true)];
        let cursor = 8;

        const board = Array.from({ length: height },
            (_, y) => Array.from({ length: width },
                (_, x) => data.getUint8(cursor + (x + y * width))
            )
        );
        cursor += width * height;

        const nLines = data.getUint32(cursor, true);
        cursor += 4;

        const lines = Array.from({ length: nLines }, function () {
            const [x, y, dx, dy] = [data.getUint8(cursor), data.getUint8(cursor + 1), data.getInt8(cursor + 2), data.getInt8(cursor + 3)];
            cursor += 4;
            return [[x, y], [dx, dy]];
        });

        const my_turn = data.getUint8(cursor);

        callback({
            width, height, board, my_turn, lines,

            get: function ({ x, y }) {
                if (x < 0 || x >= this.width || y < 0 || y >= this.height) {
                    return 0;
                }
                return this.board[y][x];
            },

            set: function ({ x, y }, value) {
                if (x < 0 || x >= this.width || y < 0 || y >= this.height) {
                    return;
                }
                this.board[y][x] = value;
            },
        });
    });
};
