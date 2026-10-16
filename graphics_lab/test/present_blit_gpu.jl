using GeometryBasics: Vec4f
using Lava
using Test
using LuxelGraphics
using Vulkan

if !isdefined(@__MODULE__, :LavaAdapter)
    include(joinpath(@__DIR__, "..", "src", "LavaAdapter.jl"))
end
using .LavaAdapter

# The presented frame reaches the swapchain by a GPU image blit when the device
# supports it. That blit must store the same bytes the CPU path stores: readback,
# clamp, upload, then Lava's fullscreen blit into an sRGB target.
@testset "GPU present blit matches the CPU present path" begin
    state = LavaAdapter.backend()
    context = state.context
    srgb = Vulkan.FORMAT_B8G8R8A8_SRGB
    @test LavaAdapter._gpu_present_supported(state, Vulkan.FORMAT_R32G32B32A32_SFLOAT, srgb)

    # Odd, non-square extent; one channel steps through every 8-bit code, the
    # others run past both ends of the unorm range.
    width, height = 257, 67
    values = Vector{Vec4f}(undef, width * height)
    for x in 0:(width-1), y in 0:(height-1)
        ramp = Float32(x * height + y) / Float32(width * height - 1)
        across = Float32(x) / Float32(width - 1)
        down = Float32(y) / Float32(height - 1)
        values[x*height+y+1] = Vec4f(ramp, 1.4f0 * across - 0.2f0, down, 2.5f0 * down - 0.75f0)
    end
    source = Lava.LavaFramebuffer(width, height; ctx=context, depth=false,
        color_format=Vulkan.FORMAT_R32G32B32A32_SFLOAT)
    source_values = Lava.LavaArray{Vec4f,1}(undef, (width * height,))
    Lava.upload!(source_values, values)
    Lava.blit!(state.queue, Lava.OffscreenTarget(source), source_values)

    # GPU path, into an sRGB image that also takes transfer writes, as the
    # swapchain does.
    image = Lava.image_2d(context, width, height, srgb, Lava.COLOR_USAGE | Vulkan.IMAGE_USAGE_TRANSFER_DST_BIT)
    memory = Lava.alloc_image_memory(context, image)
    gpu_target = Lava.LavaFramebuffer(width, height, image, memory, Lava.image_view(context, image, srgb), srgb,
        nothing, nothing, nothing, Vulkan.FORMAT_D32_SFLOAT, context, 1, nothing, nothing, nothing)
    batch = Lava.ensure_active_batch!(state.queue)
    LavaAdapter._blit_color_image!(batch.cmd_buf, source, image)
    Lava.pin!(batch, source)
    gpu = Lava.readback_framebuffer(gpu_target)

    # CPU path, from the same source (left in COLOR_ATTACHMENT_OPTIMAL by the blit).
    pixels = Lava.readback_framebuffer(source)
    @test all(pixels[x+1, y+1] == Tuple(values[x*height+y+1]) for x in 0:(width-1), y in 0:(height-1))
    cpu_values = Lava.LavaArray{Vec4f,1}(undef, (width * height,))
    Lava.upload!(cpu_values, LavaAdapter._present_pixel_data(pixels, width, height))
    cpu_target = Lava.LavaFramebuffer(width, height; ctx=context, depth=false, color_format=srgb)
    Lava.blit!(state.queue, Lava.OffscreenTarget(cpu_target), cpu_values)
    cpu = Lava.readback_framebuffer(cpu_target)

    @test size(gpu) == size(cpu) == (width, height)
    deltas = [abs(Int(gpu[i][c]) - Int(cpu[i][c])) for i in eachindex(gpu), c in 1:4]
    differing = count(!iszero, deltas)
    @info "GPU vs CPU present bytes" differing max_delta = maximum(deltas) channels = length(deltas)
    @test maximum(deltas) <= 1
end

# A window the compositor or the user resizes keeps presenting: the GPU blit and
# the CPU fallback both scale the camera-extent capture into the swapchain, and
# with the same (nearest) rule they store the same bytes.
@testset "present into a resized window" begin
    state = LavaAdapter.backend()
    context = state.context
    srgb = Vulkan.FORMAT_B8G8R8A8_SRGB

    tiny = [(Float32(x), Float32(y), 0.0f0, 1.0f0) for x in 0:1, y in 0:1]
    @test LavaAdapter._present_pixel_data(tiny, 2, 2, 4, 4) ==
          [Vec4f(min(x ÷ 2, 1), min(y ÷ 2, 1), 0, 1) for x in 0:3 for y in 0:3]

    width, height = 64, 40
    window_width, window_height = 101, 77
    values = [Vec4f(Float32(x) / (width - 1), Float32(y) / (height - 1), Float32((x + y) % 2), 1)
              for x in 0:(width-1) for y in 0:(height-1)]
    source = Lava.LavaFramebuffer(width, height; ctx=context, depth=false,
        color_format=Vulkan.FORMAT_R32G32B32A32_SFLOAT)
    source_values = Lava.LavaArray{Vec4f,1}(undef, (width * height,))
    Lava.upload!(source_values, values)
    Lava.blit!(state.queue, Lava.OffscreenTarget(source), source_values)

    @test LavaAdapter._present_blit_filter(state, source, width, height) == Vulkan.FILTER_NEAREST
    @test LavaAdapter._present_blit_filter(state, source, window_width, window_height) in
          (Vulkan.FILTER_LINEAR, Vulkan.FILTER_NEAREST)

    image = Lava.image_2d(context, window_width, window_height, srgb, Lava.COLOR_USAGE | Vulkan.IMAGE_USAGE_TRANSFER_DST_BIT)
    memory = Lava.alloc_image_memory(context, image)
    gpu_target = Lava.LavaFramebuffer(window_width, window_height, image, memory, Lava.image_view(context, image, srgb), srgb,
        nothing, nothing, nothing, Vulkan.FORMAT_D32_SFLOAT, context, 1, nothing, nothing, nothing)
    batch = Lava.ensure_active_batch!(state.queue)
    LavaAdapter._blit_color_image!(batch.cmd_buf, source, image, window_width, window_height;
        filter=Vulkan.FILTER_NEAREST)
    Lava.pin!(batch, source)
    gpu = Lava.readback_framebuffer(gpu_target)

    pixels = Lava.readback_framebuffer(source)
    cpu_values = Lava.LavaArray{Vec4f,1}(undef, (window_width * window_height,))
    Lava.upload!(cpu_values, LavaAdapter._present_pixel_data(pixels, width, height, window_width, window_height))
    cpu_target = Lava.LavaFramebuffer(window_width, window_height; ctx=context, depth=false, color_format=srgb)
    Lava.blit!(state.queue, Lava.OffscreenTarget(cpu_target), cpu_values)
    cpu = Lava.readback_framebuffer(cpu_target)

    @test size(gpu) == size(cpu) == (window_width, window_height)
    # Where a destination pixel centre lands exactly on a source texel edge,
    # the GPU's nearest rule may round either way; everywhere else it must agree.
    tie(index, out, extent) = rem((2index + 1) * extent, 2out) == 0
    compared = [I for I in CartesianIndices(gpu)
                if !tie(I[1] - 1, window_width, width) && !tie(I[2] - 1, window_height, height)]
    differing = count(I -> maximum(abs.(Int.(gpu[I]) .- Int.(cpu[I]))) > 1, compared)
    @info "scaled GPU vs CPU present" differing compared = length(compared) pixels = length(gpu)
    @test length(compared) > length(gpu) ÷ 2
    @test differing == 0
end

include(joinpath(@__DIR__, "support", "camera_fixture.jl"))

# The session window is resizable and KDE/Wayland has resized it mid-session on
# its own. Presenting must follow the new swapchain extent on both paths instead
# of failing on a camera/window size mismatch.
@testset "presented session survives a window resize" begin
    state = LavaAdapter.backend()
    packet = camera_test_scene()
    width, height = Int(packet.camera.width_px), Int(packet.camera.height_px)
    camera_test_clear_view!(state)
    LavaAdapter.open_window_session(state, width, height; vsync=false)
    try
        window = LavaAdapter.WINDOW_SESSION_REF[].window
        @test LavaAdapter.render_window_frames!(state, packet, nothing, 1).frames_presented == 1
        Lava.GLFW.SetWindowSize(window.handle, width + 37, height + 9)
        for _ in 1:50
            Lava.GLFW.PollEvents()
            Lava.GLFW.GetFramebufferSize(window.handle) != (width, height) && break
            Base.Libc.systemsleep(0.02)
        end
        for present in (:auto, :cpu)
            result = LavaAdapter.render_window_frames!(state, packet, nothing, 2; present)
            @test result.frames_presented == 2
            if present == :auto && state.gpu_timestamp_supported
                for key in ("gpu_frame", "gpu_shadow", "gpu_reflection", "gpu_prepare",
                            "gpu_scene_raster", "gpu_resolve", "gpu_overlay", "gpu_present_blit")
                    @test haskey(result.stage_times_us, key)
                    @test length(result.stage_times_us[key]) == result.frames_presented
                end
                @test all(>(0), result.stage_times_us["gpu_frame"])
            end
        end
        @info "resized window" camera = (width, height) window = size(window)
        @test size(window) != (width, height)
    finally
        @test LavaAdapter.close_window_session!(state).closed
    end
end
