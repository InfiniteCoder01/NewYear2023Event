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
            return [x, y];
        });

        callback({
            width, height, field,
            zoneMeter, zoneMax, zoneLines,
            tetromino,
            ttt: () => {
                console.log(tetromino);
            }
        });
    });
};

export default processMessage;