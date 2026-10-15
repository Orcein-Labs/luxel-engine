using Test, JSON3, LuxelGraphics

@testset "bounded native light packet admission" begin
    sun = (light_id="sun", kind=(kind="directional", direction_xyz=(0, -1, 0)), color_rgb=(1, 1, 1), intensity=1)
    point(i; range=5.0, intensity=18.0) = (light_id="point-$i", kind=(kind="point", position_xyz_m=(1, 2, 3), range_m=range), color_rgb=(1, 0.4, 0.1), intensity=intensity)
    parse_lights(values) = LuxelGraphics._parse_lights(JSON3.read(JSON3.write(values)))
    for count in 0:4
        @test length(parse_lights([sun, (point(i) for i in 1:count)...])) == count + 1
    end
    for bad in ([sun, (point(i) for i in 1:5)...], [point(1)], [sun, merge(sun, (light_id="sun2",))],
                [sun, point(1; range=0.0)], [sun, point(1; intensity=-1.0)], [sun, point(1), point(1)])
        @test_throws LuxelGraphics.ProtocolError parse_lights(bad)
    end
end
