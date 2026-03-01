#!/bin/bash

glslc shaders/main.vert -o assets/main.vert.spv
glslc shaders/main.frag -o assets/main.frag.spv

glslc shaders/shadow.vert -o assets/shadow.vert.spv
glslc shaders/shadow.frag -o assets/shadow.frag.spv

glslc shaders/skybox.vert -o assets/skybox.vert.spv
glslc shaders/skybox.frag -o assets/skybox.frag.spv

glslc shaders/main2d.vert -o assets/main2d.vert.spv
glslc shaders/main2d.frag -o assets/main2d.frag.spv

