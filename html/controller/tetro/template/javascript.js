// This is a simple implementation, showcasing the API. Improve it, or write your own!
// You can see all methods provided here: https://github.com/InfiniteCoder01/NewYear2024Event/blob/master/html/controller/tetro/api.js
// socket.send will send your move to the server. Note, that server will only allow
// moves to go 100 times per second. But you can send more, they will just be delayed
// Server API:
// CW - Turn tetromino Clockwise
// CCW - Turn tetromino Counter Clockwise
// Left - Move tetromino to the left
// Right - Move tetromino to the right
// Zone - activate the zone mode
// FastFall - If in zone mode, will drop the tetromino down. Otherwise, will make tetromino fall faster
// SlowFall - Makes tetromino fall at normal speed if there was FastFall before. Note, that the speed doesn't reset after tetromino gets placed.

// Pro tip: you can monitor your bot in near-realtime in controller.
// Just press a gamepad button on the title bar!

// Scoring function, used to test every possible move
function scoreFunction(game) {
    let lastY = game.tetromino.y;
    game.drop();
    game.place();

    let dropY = game.tetromino.y;

    let completeLines = game.complete_lines();

    let totalHeight = 0, maxHeight = 0, bumpines = 0;
    let lastHeight = null;
    for (let x = 0; x < game.width; x++) {
        let height = 0;
        for (let y = 0; y < game.height; y++) {
            if (game.get({ x, y })) {
                height = game.height - y;
                break;
            }
        }
        totalHeight += height - completeLines.length;
        maxHeight = Math.max(height, maxHeight);
        if (lastHeight != null) {
            bumpines += Math.abs(height - lastHeight);
        }
        lastHeight = height;
    }

    let holes = 0;
    for (let y = 1; y < game.height; y++) {
        for (let x = 0; x < game.width; x++) {
            if (!game.get({ x, y }) && game.get({ x, y: y - 1 })) {
                holes++;
            }
        }
    }

    const score = maxHeight * 0.0
        + totalHeight * -0.510066
        + completeLines.length * 0.760666
        + holes * -0.35663
        + bumpines * -0.184483; // Weights are taken from here: https://codemyroad.wordpress.com/2013/04/14/tetris-ai-the-near-perfect-player/

    // Cleanup, so we can make more simulations
    game.unplace();
    let y = game.tetromino.y;
    game.tetromino.y = lastY;

    return [score, maxHeight];
}

// Helper function to construct a move object
function move(game, rotation) {
    const [score, maxHeight] = scoreFunction(game);
    return {
        x: game.tetromino.x,
        rotation,
        score,
        maxHeight,
    };
}

// Helper funciton to compare moves
function bestMove(oldMove, newMove) {
    // Note, that we will keep old move if scores are equal,
    // so we don't jump between two possible moves with the same score
    if (newMove.score > oldMove.score) {
        return newMove;
    }
    return oldMove;
}

function callback(game) {
    let best = move(game, 0);

    // Try every possible turn
    for (let rotation = 0; rotation < 4; rotation++) {
        // Turn without moving
        best = bestMove(best, move(game, rotation));

        // Try moving left
        let lastX = game.tetromino.x;
        while (game.try_move(-1)) { // -1 is for left. try_* functions return true, if the motion was successful
            best = bestMove(best, move(game, rotation));
        }
        game.tetromino.x = lastX; // Cleanup to continue simulations

        // Try moving right
        lastX = game.tetromino.x;
        while (game.try_move(1)) { // 1 is for right
            best = bestMove(best, move(game, rotation));
        }
        game.tetromino.x = lastX;

        if (!game.try_turn(false)) break; // true for counter clockwise
    }

    let speedup = false;
    if (best.rotation > 0) {
        socket.send(best.rotation > 2 ? "CCW" : "CW");
    } else if (best.x != game.tetromino.x) {
        socket.send(best.x < game.tetromino.x ? "Left" : "Right");
    } else if (best.maxHeight < game.height - 3) { // Protection from zone lines filling too much space
        socket.send("FastFall"); // Will initiate fall when in zone mode
    }

    if (game.zoneMeter >= game.zoneMax * 0.4) {
        socket.send("Zone");
    }
}

print("Connecting...");
connect(callback);