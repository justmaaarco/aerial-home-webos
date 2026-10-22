# Public demo scenery

`original-coast.png` is an original AI-generated scenic still made with the built-in imagegen tool for this project's browser showcase. It is not an Apple aerial, a video frame or a photograph of a user's location. Demo icons are the project's original sample artwork. The scene is not bundled in the TV package.

Generated with this prompt:

> Use case: photorealistic-natural. Asset type: original scenic background for an open-source television launcher demonstration, 16:9 wide landscape. Create a beautiful cinematic aerial photograph of an imaginary wild Nordic coastline at blue hour: dark forested mountain headland sweeping diagonally into a deep teal fjord, distant layered peaks, subtle peach sunrise on the horizon, wisps of sea mist, luminous tiny white surf along the rocky shoreline. Natural realistic textures, sophisticated quiet atmosphere, crisp landscape detail. Wide elevated drone view, restrained contrast, darker calm water and mountains in the lower half so white UI icons can read over it. No text, no logos, no watermark, no buildings, no recognizable real landmark, no app interface. Entirely original composition, do not reproduce any Apple aerial screenshot. Output a wide 16:9 scenic image only.

Run `node tools/showcase.cjs` and open the local address printed by the server. The server uses the real app in browser-preview mode and adds only a demo backdrop stylesheet. The visible footer labels captures as a browser demo; the simulated Apple scene caption is hidden so it cannot mislabel the original still as an Apple video. No TV connection is made.
