ffmpeg -i Assets/Animation/Animation.mp4 -c:v libx264 -profile:v high -level:v 3.1 -filter:v fps=30 -c:a copy Assets/Animation/AnimationV2.mp4
ffmpeg -f lavfi -i anullsrc=channel_layout=stereo:sample_rate=44100 -i Assets/Animation/AnimationV2.mp4 -c:v copy -c:a aac -shortest Assets/Animation/AnimationSilent.mp4
