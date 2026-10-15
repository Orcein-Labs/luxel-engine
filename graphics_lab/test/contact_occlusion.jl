using Base64, SHA, Test, LuxelGraphics
using GeometryBasics: Vec4f
if !isdefined(@__MODULE__, :camera_test_packet)
    include(joinpath(@__DIR__, "support", "camera_fixture.jl"))
end

@testset "contact AO geometry and depth payload helpers" begin
    projection = Vec4f(10.0f0, 10.0f0, 0.1f0, 100.0f0)
    @test LavaAdapter._contact_depth_meters(0.0f0, projection, 1.0f0) ≈ 0.1f0
    @test LavaAdapter._contact_depth_meters(1.0f0, projection, 1.0f0) ≈ 100.0f0
    @test LavaAdapter._contact_depth_meters(0.5f0, projection, 0.0f0) ≈ 50.05f0

    normal = Vec4f(0.0f0, 0.0f0, -1.0f0, 0.0f0)
    center = Vec4f(0.0f0, 0.0f0, 5.0f0, 0.0f0)
    flat_neighbors = ntuple(index -> begin
        angle = Float32(index - 1) * (Float32(pi) / 4.0f0)
        Vec4f(cos(angle), sin(angle), 5.0f0, 0.0f0)
    end, 8)
    @test LavaAdapter._contact_ao_factor(normal, center, flat_neighbors, 0.8f0, 0.02f0) == 0.0f0

    touching = ntuple(index -> begin
        angle = Float32(index - 1) * (Float32(pi) / 4.0f0)
        Vec4f(cos(angle) * 0.35f0, sin(angle) * 0.35f0, 4.75f0, 0.0f0)
    end, 8)
    floating = ntuple(index -> begin
        angle = Float32(index - 1) * (Float32(pi) / 4.0f0)
        Vec4f(cos(angle) * 0.35f0, sin(angle) * 0.35f0, 3.8f0, 0.0f0)
    end, 8)
    touching_occlusion = LavaAdapter._contact_ao_factor(normal, center, touching, 0.8f0, 0.02f0)
    floating_occlusion = LavaAdapter._contact_ao_factor(normal, center, floating, 0.8f0, 0.02f0)
    @test touching_occlusion > 0.0f0
    @test floating_occlusion == 0.0f0

    depth_bytes = LavaAdapter._contact_depth_bytes(reshape(Float32[1.0f0, 2.0f0, 3.0f0, 4.0f0], 2, 2))
    @test length(depth_bytes) == 16
    @test reinterpret(Float32, depth_bytes) == Float32[1.0f0, 2.0f0, 3.0f0, 4.0f0]
    asymmetric = Float32[
        1 5 9 13;
        2 6 10 14;
        3 7 11 15;
        4 8 12 16;
    ]
    reduced = LavaAdapter._contact_downsample_nearest(asymmetric, 2, 2)
    @test reduced == Float32[1 9; 3 11]
    @test reinterpret(Float32, LavaAdapter._contact_depth_bytes(reduced)) == Float32[1, 3, 9, 11]
    @test_throws LavaAdapter.AdapterError LavaAdapter._contact_downsample_nearest(asymmetric, 1, 2)
end

if get(ENV, "LUXEL_CONTACT_AO_GPU_TESTS", "") == "1"
    @testset "contact AO GPU controls" begin
        state = LavaAdapter.backend()
        packet = camera_test_packet()
        disabled = LavaAdapter.render_scene(packet, state)
        disabled_again = LavaAdapter.render_scene(packet, state)
        @test disabled.capture_sha256 == disabled_again.capture_sha256

        zero_strength_policy = camera_test_replace(LuxelGraphics.RenderPolicy(); contact_ao=LuxelGraphics.ContactAoPolicy(80, 0, 20))
        zero_strength = camera_test_replace(packet; packet_sha256="contact-zero", content_sha256="contact-zero",
            render_policy=zero_strength_policy)
        zero_result = LavaAdapter.render_scene(zero_strength, state)
        @test disabled.capture_base64 == zero_result.capture_base64

        active_policy = camera_test_replace(LuxelGraphics.RenderPolicy(); contact_ao=LuxelGraphics.ContactAoPolicy(80, 6000, 20))
        active_packet = camera_test_replace(packet; packet_sha256="contact-flat", content_sha256="contact-flat",
            render_policy=active_policy)
        flat_result = LavaAdapter.render_scene(active_packet, state)
        @test disabled.capture_base64 == flat_result.capture_base64
        resources_before_override = state.contact_ao_resources
        override_frame = LavaAdapter._camera_frame(camera_test_replace(active_packet.camera;
            position_xyz_m=(0.1f0, 8.0f0, 0.0f0)))
        resources_after_override = LavaAdapter._contact_ao_resources!(state, active_packet,
            override_frame, resources_before_override.source_framebuffer)
        @test resources_after_override.depth_framebuffer === resources_before_override.depth_framebuffer
        @test resources_after_override.ao_framebuffer === resources_before_override.ao_framebuffer
        @test resources_after_override.color_sampler === resources_before_override.color_sampler
        @test resources_after_override.depth_sampler === resources_before_override.depth_sampler
        @test resources_after_override.bindings === resources_before_override.bindings

        # A simple box above the flat terrain tests contact discrimination. A
        # 1.5 m lift is beyond the AO radius while touching is at the surface.
        box_positions = [
            (-0.4f0, 0.0f0, -0.4f0), (0.4f0, 0.0f0, -0.4f0),
            (0.4f0, 0.25f0, -0.4f0), (-0.4f0, 0.25f0, -0.4f0),
            (-0.4f0, 0.0f0, 0.4f0), (0.4f0, 0.0f0, 0.4f0),
            (0.4f0, 0.25f0, 0.4f0), (-0.4f0, 0.25f0, 0.4f0),
        ]
        faces = UInt32[0,2,1, 0,3,2, 4,5,6, 4,6,7, 0,1,5, 0,5,4,
                       3,7,6, 3,6,2, 0,4,7, 0,7,3, 1,2,6, 1,6,5]
        box = LuxelGraphics.MeshPacket("contact-box", box_positions,
            fill((0.0f0, 1.0f0, 0.0f0), 8), fill((0.0f0, 0.0f0), 8), faces,
            "bounds-material")
        instance(raise) = LuxelGraphics.InstancePacket("contact-box-instance", "contact-box", "bounds-material",
            LuxelGraphics.BackgroundImportance(), LuxelGraphics.TransformPacket((0.0f0, raise, 0.0f0),
                (0.0f0, 0.0f0, 0.0f0, 1.0f0), (1.0f0, 1.0f0, 1.0f0)))
        box_packet = camera_test_replace(packet; packet_sha256="contact-touch", content_sha256="contact-touch",
            meshes=[box], instances=[instance(0.0f0)], render_policy=active_policy)
        touching_off_packet = camera_test_replace(box_packet; packet_sha256="contact-touch-off", content_sha256="contact-touch-off", render_policy=LuxelGraphics.RenderPolicy())
        touching_off = LavaAdapter.render_scene(touching_off_packet, state)
        touching_result = LavaAdapter.render_scene(box_packet, state)
        depth = LavaAdapter.capture_scene_depth(box_packet, state)
        @test depth.schema == "luxel.view-depth/v1"
        @test depth.capture_id == box_packet.capture_id
        @test depth.width_px == box_packet.width_px
        @test depth.height_px == box_packet.height_px
        @test depth.depth_sha256 == "sha256:" * bytes2hex(SHA.sha256(Base64.base64decode(depth.depth_base64)))
        floating_packet = camera_test_replace(box_packet; packet_sha256="contact-float", content_sha256="contact-float",
            instances=[instance(1.5f0)])
        floating_off_packet = camera_test_replace(floating_packet; packet_sha256="contact-float-off", content_sha256="contact-float-off", render_policy=LuxelGraphics.RenderPolicy())
        floating_off = LavaAdapter.render_scene(floating_off_packet, state)
        floating_result = LavaAdapter.render_scene(floating_packet, state)
        @test_throws LavaAdapter.AdapterError LavaAdapter.capture_scene_depth(box_packet, state)
        function receiver_drop(off, on)
            before = Base64.base64decode(off.capture_base64)
            after = Base64.base64decode(on.capture_base64)
            total = 0
            for y in 1:64, x in 1:64
                world_x = (Float32(x) - 32.5f0) * 0.25f0
                world_z = (Float32(y) - 32.5f0) * 0.25f0
                # Ground ring excludes the object's projected silhouette and
                # MSAA edge pixels; each pose uses its own AO-off control.
                max(abs(world_x), abs(world_z)) > 0.5f0 || continue
                hypot(world_x, world_z) < 1.4f0 || continue
                offset = 4 * ((y - 1) * 64 + x - 1)
                for channel in 1:3
                    total += Int(before[offset + channel]) - Int(after[offset + channel])
                end
            end
            return total
        end
        touching_drop = receiver_drop(touching_off, touching_result)
        floating_drop = receiver_drop(floating_off, floating_result)
        @info "contact receiver controls" touching_drop floating_drop
        @test touching_drop > 0
        @test touching_drop > floating_drop
        @test floating_drop == 0
        output = get(ENV, "LUXEL_CONTACT_OUTPUT", joinpath(@__DIR__, "..", "..", "artifacts", "contact-occlusion-20261012"))
        mkpath(output)
        for (name, frame) in (("touch-off", touching_off), ("touch-on", touching_result), ("float-off", floating_off), ("float-on", floating_result))
            bytes = Base64.base64decode(frame.capture_base64)
            open(joinpath(output, name * ".ppm"), "w") do io
                write(io, "P6\n64 64\n255\n")
                for offset in 1:4:length(bytes)
                    write(io, bytes[offset], bytes[offset + 1], bytes[offset + 2])
                end
            end
        end

    end
else
    @test_skip false # GPU semantics are run explicitly under /tmp/gesso-gpu.lock.
end
