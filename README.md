Raytrs
---
Simple multithreaded software raytracing engine in rust, made for fun.<br>
I'm still learning, so any tips or suggestions are appreciated.<br>

Scenes are stored in the json format,
and the examples in `example_scenes/` should give you enough context to make your own scenes.
Loading custom obj files is supported, but textures are not.

`benchmark.sh` is a simple script that renders the example scenes
(it must be run inside of the repo directory) and gives a score (in arbitrary units)
based on the time they took.
The current version, on my computer, tends to score around 93

	Usage: raytrs [OPTION]...

		-h, --help                      show this message
		-q, --quiet                     quiet mode, only print render time to stdout
		-s, --scene <filename.json>     set scene file. if no scene is provided, a
                                    	very simple example will be rendered.
		-o, --output <filename.png>     set output file. defaults to render.png
		-t, --threads <# of threads>    set number of threads used. should be >= the
                                        number of logical cores in your system,
										defaults to 32
		-r, --resolution <WIDTHxHEIGHT> set image dimensions. defaults to 256x256


![spheres, shading, reflections, obj importing, shadows and multiple light sources](demo.png "demo image")
