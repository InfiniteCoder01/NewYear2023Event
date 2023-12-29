const apiName = "place";
const palette = [
    "000000",
    "55415f",
    "646964",
    "d77355",
    "508cd7",
    "64b964",
    "e6c86e",
    "dcf5ff",
];

const processMessage = (msg, callback) => {
    msg.data.arrayBuffer().then(buffer => {
        const data = new DataView(buffer);

        const [width, height] = [data.getUint32(0, true), data.getUint32(4, true)];
        let cursor = 8;

        const pixels = Array.from({ length: height },
            (_, y) => Array.from({ length: width },
                (_, x) => data.getUint32(cursor + (x + y * width) * 4, true).toString(16).padStart(6, "0")
            )
        );

        callback({
            width, height, pixels,
            get: function ({ x, y }) {
                if (x < 0 || x >= this.width || y < 0 || y >= this.height) {
                    return 0;
                }
                return this.pixels[y][x];
            },

            set: function ({ x, y }, value) {
                if (x < 0 || x >= this.width || y < 0 || y >= this.height) {
                    return;
                }
                this.pixels[y][x] = value;
            },
        });
    });
};
