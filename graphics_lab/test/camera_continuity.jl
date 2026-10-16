include(joinpath(@__DIR__, "support", "camera_fixture.jl"))

@testset "moving camera capture and resource continuity" begin
    state = LavaAdapter.backend()
    packet_a = camera_test_scene()
    packet_b = camera_test_replace(packet_a; camera=camera_test_replace(packet_a.camera;
        position_xyz_m=(40f0, 8f0, 0f0)))
    camera_test_clear_view!(state)
    first_a = LavaAdapter.render_scene(packet_a, state)
    mesh_a = state.mesh_resources
    terrain_a, shadow_a, ibl_a = state.terrain_resources, state.shadow_resources, state.ibl_resources
    warm_b = LavaAdapter.render_scene(packet_b, state)
    mesh_b = state.mesh_resources
    @test mesh_a !== mesh_b
    @test only(mesh_b.batches).positions === only(mesh_a.batches).positions
    @test only(mesh_b.batches).normals === only(mesh_a.batches).normals
    @test only(mesh_b.batches).uvs === only(mesh_a.batches).uvs
    @test only(mesh_b.batches).tangents === only(mesh_a.batches).tangents
    @test only(mesh_b.batches).translations !== only(mesh_a.batches).translations
    @test state.terrain_resources === terrain_a
    @test state.shadow_resources === shadow_a
    @test state.ibl_resources === ibl_a
    @test mesh_b.visible_instance_ids == ["instance-40.0"]
    again_b = LavaAdapter.render_scene(packet_b, state)
    @test again_b.capture_sha256 == warm_b.capture_sha256
    @test again_b.telemetry.upload_bytes == 0
    @test state.mesh_resources === mesh_b

    camera_test_clear_view!(state)
    cold_b = LavaAdapter.render_scene(packet_b, state)
    @test warm_b.capture_sha256 == cold_b.capture_sha256
    @test warm_b.telemetry.upload_bytes < cold_b.telemetry.upload_bytes
    println("B warm/cold capture: ", warm_b.capture_sha256,
        " uploads: ", warm_b.telemetry.upload_bytes, "/", cold_b.telemetry.upload_bytes)

    # Motion that retains the visible set should keep the entire draw batch,
    # but must reproject screen-space overlay vertices.
    before_mesh, before_overlay = state.mesh_resources, state.overlay_resources
    packet_pan = camera_test_replace(packet_b; camera=camera_test_replace(packet_b.camera;
        position_xyz_m=(41f0, 8f0, 0f0)))
    pan = LavaAdapter.render_scene(packet_pan, state)
    @test state.mesh_resources.batches === before_mesh.batches
    @test state.overlay_resources !== before_overlay
    @test pan.capture_sha256 != cold_b.capture_sha256
    camera_test_clear_view!(state)
    @test LavaAdapter.render_scene(packet_pan, state).capture_sha256 == pan.capture_sha256

    # Empty visible sets must never draw the preceding view's instances or markers.
    packet_empty = camera_test_replace(packet_b; camera=camera_test_replace(packet_b.camera;
        position_xyz_m=(80f0, 8f0, 0f0)))
    empty_view = LavaAdapter.render_scene(packet_empty, state)
    @test empty_view.telemetry.visible_instance_count == 0
    @test empty_view.telemetry.mesh_vertex_count == 0
    camera_test_clear_view!(state)
    @test LavaAdapter.render_scene(packet_empty, state).capture_sha256 == empty_view.capture_sha256

    # Reflection culling has its own view cache, with the same continuity rule.
    frame_a = LavaAdapter._camera_frame(packet_a.camera)
    frame_b = LavaAdapter._camera_frame(packet_b.camera)
    reflection_a = LavaAdapter._mesh_resources!(state, packet_a, frame_a; cache=:reflection)
    reflection_b = LavaAdapter._mesh_resources!(state, packet_a, frame_b; cache=:reflection)
    @test reflection_b.visible_instance_ids == ["instance-40.0"]
    @test only(reflection_b.batches).positions === only(reflection_a.batches).positions
    @test LavaAdapter._mesh_resources!(state, packet_a, frame_b; cache=:reflection) === reflection_b

    # Mesh streams are packet-scoped: the main and reflection caches share them.
    main_a = LavaAdapter._mesh_resources!(state, packet_a, frame_a; cache=:main)
    reflection_c = LavaAdapter._mesh_resources!(state, packet_a, frame_b; cache=:reflection)
    @test only(reflection_c.batches).positions === only(main_a.batches).positions
    # With the packet's geometry warm, a main-cache rebuild uploads only the
    # per-instance streams: 7 Vec4f arrays of one instance each.
    state.mesh_resources = nothing
    upload_before = state.upload_bytes
    main_b = LavaAdapter._mesh_resources!(state, packet_a, frame_b; cache=:main)
    @test main_b.visible_instance_ids == ["instance-40.0"]
    @test state.upload_bytes - upload_before == 7 * sizeof(LavaAdapter.Vec4f) * 1

    # A new packet identity invalidates every mesh stream even if IDs and view
    # membership match. Authority rejects unsealed source changes upstream.
    original_mesh = only(packet_a.meshes)
    for (label, replacement) in (
        (:positions, camera_test_replace(original_mesh; positions_m=[(-1f0, 1f0, -2f0), (2f0, 1f0, -2f0), (0f0, 1f0, 2f0)])),
        (:normals, camera_test_replace(original_mesh; normals=fill((0.1f0, 1f0, 0f0), 3))),
        (:uvs, camera_test_replace(original_mesh; uv0=fill((0.25f0, 0.5f0), 3))),
        (:tangents, camera_test_replace(original_mesh; tangents=fill((1f0, 0f0, 0f0, 1f0), 3))),
        (:indices, camera_test_replace(original_mesh; indices=UInt32[0, 1, 2])),
    )
        prior = LavaAdapter._mesh_resources!(state, packet_a, frame_a)
        changed = camera_test_replace(packet_a; content_sha256="content-$label", meshes=[replacement])
        fresh = LavaAdapter._mesh_resources!(state, changed, frame_a)
        @test only(fresh.batches).positions !== only(prior.batches).positions
        @test only(fresh.batches).normals !== only(prior.batches).normals
        @test only(fresh.batches).uvs !== only(prior.batches).uvs
        @test only(fresh.batches).tangents !== only(prior.batches).tangents
    end

    # Exercise the actual presented camera_override entry point, retaining the
    # original packet. Its resources must produce the same B as the cold oracle.
    camera_test_clear_view!(state)
    LavaAdapter.open_window_session(state, 64, 64; vsync=false)
    try
        @test LavaAdapter.render_window_frames!(state, packet_a, nothing, 1).frames_presented == 1
        @test LavaAdapter.render_window_frames!(state, packet_a, packet_b.camera, 1).frames_presented == 1
        @test state.mesh_resources.visible_instance_ids == ["instance-40.0"]
        @test LavaAdapter.render_scene(packet_b, state).capture_sha256 == cold_b.capture_sha256
    finally
        @test LavaAdapter.close_window_session!(state).closed
    end
    camera_test_clear_view!(state)
    @test LavaAdapter.render_scene(packet_a, state).capture_sha256 == first_a.capture_sha256
    println("A cold/rebuild capture: ", first_a.capture_sha256)
    historical = camera_test_replace(packet_b; overlays=LuxelGraphics.OverlayValue[
        camera_test_replace(overlay; position_xyz_m=(overlay.position_xyz_m[1], 2f0, 3f0))
        for overlay in packet_b.overlays])
    camera_test_clear_view!(state)
    println("historical B cold capture: ", LavaAdapter.render_scene(historical, state).capture_sha256)
end
