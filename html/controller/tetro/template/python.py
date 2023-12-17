def client(game):
    print(game.try_move(-1))
    sendMessage("Left")

registerClient(client)