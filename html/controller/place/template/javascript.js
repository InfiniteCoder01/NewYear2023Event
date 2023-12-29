// This is a simple painter, showcasing the API. Get creative!
// You can see all methods provided here: https://github.com/InfiniteCoder01/NewYear2024Event/blob/master/html/controller/place/api.js
// socket.send will send your pixel to the server.
// Server API:
// "x y color", where X and Y are integer coordinates of the pixel and COLOR is a 6-digit hex color, for example "dcf5ff".

// Pro tip: you can monitor the board in near-realtime in controller.
// Just press a gamepad button on the navigation bar!

let [x, y] = [0, 0];
let place = null;
const color = palette[Math.floor(Math.random() * palette.length)]; // Pick a random color from the palette

setInterval(function () {
    if (place == null) return;
    // While the pixel is the color we want, pick another random one.
    // `it` variable is to protect you from infinite loop, which might freeze the browser
    let it = 0;
    do {
        x = Math.floor(Math.random() * place.width);
        y = Math.floor(Math.random() * place.height);
        it++;
    } while (place.get({ x, y }) == color && it < 1000);

    socket.send(`${x} ${y} ${color}`);
    place.set({ x, y }, color); // Will still be overwritten by our callback.
}, 1000);

print("Ready.");
connect(state => place = state);