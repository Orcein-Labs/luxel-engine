# TetKernels — vendor-neutral KernelAbstractions ports of every device kernel
# in gpu/ (spirals E, F, H, H2, I and probe3).
#
# One source, any backend: CPU(), CUDABackend() (CUDA.jl), ROCBackend()
# (AMDGPU.jl). The gpu/ scripts stay as recorded evidence; these ports keep
# their argument order and arithmetic verbatim (same expression order, same
# f32 literals, same 1-based index math) so parity.jl can demand bit-identical
# output against the original @cuda kernels on the same device.
#
# The explicit `i <= n` guards are kept although KernelAbstractions also
# bounds-checks the ndrange: launch with ndrange = n and they are a no-op.
module TetKernels

using KernelAbstractions

export deform_corners!, reconstruct!, pair_dev!, fused_deform_reconstruct!,
       fused_batched_wind!, reconstruct_batched!, rot_direct!, lbs!,
       morph_lerp!, fused_batched_shared!, probe_div!, KERNELS

const F_PX = 500.0f0
const D_CAM = 5.0f0

# fam: 1 wind, 2 twist, 3 fold4 — spiral E/F/H/probe3
@kernel function deform_corners!(ox, oy, oz, @Const(cx), @Const(cy), @Const(cz), n, s, fam, A, phi)
    i = @index(Global, Linear)
    if i <= n
        x = cx[i]; y = cy[i]; z = cz[i]
        if fam == 1
            t = clamp(y / s, 0f0, 1f0)
            lean = A * s * t * t * sin(phi + 2.1f0 * x / s + 1.3f0 * z / s)
            ox[i] = x + lean; oy[i] = y; oz[i] = z
        elseif fam == 2
            θ = A * (y / s)
            c = cos(θ); sn = sin(θ)
            ox[i] = c * x - sn * y; oy[i] = sn * x + c * y; oz[i] = z
        else
            ox[i] = x; oy[i] = y; oz[i] = z + A * s * tanh(3f0 * y / s)
        end
    end
end

# fixed-order weighted sum, mirrors TetDeform pairing — spiral E/F/H
@kernel function reconstruct!(px, py, pz, @Const(ox), @Const(oy), @Const(oz), @Const(idx), @Const(w), n)
    i = @index(Global, Linear)
    if i <= n
        i4 = 4 * (i - 1)
        a = idx[i4+1]; b = idx[i4+2]; c = idx[i4+3]; d = idx[i4+4]
        w1 = w[i4+1]; w2 = w[i4+2]; w3 = w[i4+3]; w4 = w[i4+4]
        px[i] = w2 * ox[b] + w3 * ox[c] + w4 * ox[d] + w1 * ox[a]
        py[i] = w2 * oy[b] + w3 * oy[c] + w4 * oy[d] + w1 * oy[a]
        pz[i] = w2 * oz[b] + w3 * oz[c] + w4 * oz[d] + w1 * oz[a]
    end
end

# one thread per TE pair: f32 screen-space deviation of deformed corners — spiral E
@kernel function pair_dev!(dev, @Const(ox), @Const(oy), @Const(oz), @Const(pa), @Const(pb), np)
    i = @index(Global, Linear)
    if i <= np
        ia = pa[i]; ib = pb[i]
        dx = ox[ia] - ox[ib]; dy = oy[ia] - oy[ib]; dz = oz[ia] - oz[ib]
        dist = sqrt(dx * dx + dy * dy + dz * dz)
        dev[i] = (F_PX / D_CAM) * dist
    end
end

# deform the 4 corners in registers, then reconstruct — spiral H
@kernel function fused_deform_reconstruct!(px, py, pz, @Const(cx), @Const(cy), @Const(cz), @Const(idx), @Const(w), n, s, fam, A, phi)
    i = @index(Global, Linear)
    if i <= n
        i4 = 4 * (i - 1)
        a = idx[i4+1]; b = idx[i4+2]; c = idx[i4+3]; d = idx[i4+4]
        w1 = w[i4+1]; w2 = w[i4+2]; w3 = w[i4+3]; w4 = w[i4+4]
        ax = cx[a]; ay = cy[a]; az = cz[a]
        bx = cx[b]; by = cy[b]; bz = cz[b]
        cx2 = cx[c]; cy2 = cy[c]; cz2 = cz[c]
        dx = cx[d]; dy = cy[d]; dz = cz[d]
        if fam == 1
            t = clamp(ay / s, 0f0, 1f0)
            ax = ax + A * s * t * t * sin(phi + 2.1f0 * ax / s + 1.3f0 * az / s)
            t = clamp(by / s, 0f0, 1f0)
            bx = bx + A * s * t * t * sin(phi + 2.1f0 * bx / s + 1.3f0 * bz / s)
            t = clamp(cy2 / s, 0f0, 1f0)
            cx2 = cx2 + A * s * t * t * sin(phi + 2.1f0 * cx2 / s + 1.3f0 * cz2 / s)
            t = clamp(dy / s, 0f0, 1f0)
            dx = dx + A * s * t * t * sin(phi + 2.1f0 * dx / s + 1.3f0 * dz / s)
        elseif fam == 2
            θ = A * (ay / s); cc = cos(θ); ss = sin(θ)
            nx = cc * ax - ss * ay; ay = ss * ax + cc * ay; ax = nx
            θ = A * (by / s); cc = cos(θ); ss = sin(θ)
            nx = cc * bx - ss * by; by = ss * bx + cc * by; bx = nx
            θ = A * (cy2 / s); cc = cos(θ); ss = sin(θ)
            nx = cc * cx2 - ss * cy2; cy2 = ss * cx2 + cc * cy2; cx2 = nx
            θ = A * (dy / s); cc = cos(θ); ss = sin(θ)
            nx = cc * dx - ss * dy; dy = ss * dx + cc * dy; dx = nx
        else
            az = az + A * s * tanh(3f0 * ay / s)
            bz = bz + A * s * tanh(3f0 * by / s)
            cz2 = cz2 + A * s * tanh(3f0 * cy2 / s)
            dz = dz + A * s * tanh(3f0 * dy / s)
        end
        px[i] = w2 * bx + w3 * cx2 + w4 * dx + w1 * ax
        py[i] = w2 * by + w3 * cy2 + w4 * dy + w1 * ay
        pz[i] = w2 * bz + w3 * cz2 + w4 * dz + w1 * az
    end
end

# batched instances sharing one cage, wind only — spiral H2
@kernel function fused_batched_wind!(px, py, pz, @Const(cx), @Const(cy), @Const(cz), @Const(idx), @Const(w), nV, ntot, s, A, phi)
    i = @index(Global, Linear)
    if i <= ntot
        g = (i - 1) ÷ nV
        j = i - g * nV
        i4 = 4 * (j - 1)
        a = idx[i4+1]; b = idx[i4+2]; cc = idx[i4+3]; d = idx[i4+4]
        w1 = w[i4+1]; w2 = w[i4+2]; w3 = w[i4+3]; w4 = w[i4+4]
        ax = cx[a]; ay = cy[a]; az = cz[a]
        bx = cx[b]; by = cy[b]; bz = cz[b]
        ex = cx[cc]; ey = cy[cc]; ez = cz[cc]
        fx2 = cx[d]; fy2 = cy[d]; fz2 = cz[d]
        t = clamp(ay / s, 0f0, 1f0)
        ax = ax + A * s * t * t * sin(phi + 2.1f0 * ax / s + 1.3f0 * az / s)
        t = clamp(by / s, 0f0, 1f0)
        bx = bx + A * s * t * t * sin(phi + 2.1f0 * bx / s + 1.3f0 * bz / s)
        t = clamp(ey / s, 0f0, 1f0)
        ex = ex + A * s * t * t * sin(phi + 2.1f0 * ex / s + 1.3f0 * ez / s)
        t = clamp(fy2 / s, 0f0, 1f0)
        fx2 = fx2 + A * s * t * t * sin(phi + 2.1f0 * fx2 / s + 1.3f0 * fz2 / s)
        px[i] = w2 * bx + w3 * ex + w4 * fx2 + w1 * ax
        py[i] = w2 * by + w3 * ey + w4 * fy2 + w1 * ay
        pz[i] = w2 * bz + w3 * ez + w4 * fz2 + w1 * az
    end
end

# batched reconstruct over per-instance deformed corner blocks — spiral I
@kernel function reconstruct_batched!(px, py, pz, @Const(ox), @Const(oy), @Const(oz), @Const(idx), @Const(w), nV, nT, ntot)
    i = @index(Global, Linear)
    if i <= ntot
        g = (i - 1) ÷ nV
        j = i - g * nV
        coff = g * 4 * nT
        i4 = 4 * (j - 1)
        a = idx[i4+1] + coff; b = idx[i4+2] + coff
        c = idx[i4+3] + coff; d = idx[i4+4] + coff
        w1 = w[i4+1]; w2 = w[i4+2]; w3 = w[i4+3]; w4 = w[i4+4]
        px[i] = w2 * ox[b] + w3 * ox[c] + w4 * ox[d] + w1 * ox[a]
        py[i] = w2 * oy[b] + w3 * oy[c] + w4 * oy[d] + w1 * oy[a]
        pz[i] = w2 * oz[b] + w3 * oz[c] + w4 * oz[d] + w1 * oz[a]
    end
end

# direct rigid rotation comparator — spiral I
@kernel function rot_direct!(px, py, pz, @Const(vx), @Const(vy), @Const(vz), ntot, c, s)
    i = @index(Global, Linear)
    if i <= ntot
        x = vx[i]; y = vy[i]
        px[i] = c * x - s * y
        py[i] = s * x + c * y
        pz[i] = vz[i]
    end
end

# 4-bone linear blend skinning comparator — spiral I
@kernel function lbs!(px, py, pz, @Const(vx), @Const(vy), @Const(vz), @Const(bidx), @Const(bw), @Const(pal_c), @Const(pal_s), n_per, ntot)
    i = @index(Global, Linear)
    if i <= ntot
        g = (i - 1) ÷ n_per
        j = i - g * n_per
        j4 = 4 * (j - 1)
        x = vx[j]; y = vy[j]; z = vz[j]
        ox = 0f0; oy = 0f0
        for k in 1:4
            bone = bidx[j4+k]
            wgt = bw[j4+k]
            cθ = pal_c[g * 3 + bone]  # g is 0-based: instance g's palette block is [3g+1 .. 3g+3]
            sθ = pal_s[g * 3 + bone]
            ox += wgt * (cθ * x - sθ * y)
            oy += wgt * (sθ * x + cθ * y)
        end
        px[i] = ox; py[i] = oy; pz[i] = z
    end
end

# morph-target lerp comparator — spiral I
@kernel function morph_lerp!(px, py, pz, @Const(m), ntot, alpha)
    i = @index(Global, Linear)
    if i <= ntot
        o = 6 * (i - 1)
        ax = m[o+1]; ay = m[o+2]; az = m[o+3]
        px[i] = ax + alpha * (m[o+4] - ax)
        py[i] = ay + alpha * (m[o+5] - ay)
        pz[i] = az + alpha * (m[o+6] - az)
    end
end

# batched fused rotation over a shared cage — spiral I
@kernel function fused_batched_shared!(px, py, pz, @Const(cx), @Const(cy), @Const(cz), @Const(idx), @Const(w), nV, ntot, c, s)
    i = @index(Global, Linear)
    if i <= ntot
        g = (i - 1) ÷ nV
        j = i - g * nV
        i4 = 4 * (j - 1)
        a = idx[i4+1]; b = idx[i4+2]; cc = idx[i4+3]; d = idx[i4+4]
        w1 = w[i4+1]; w2 = w[i4+2]; w3 = w[i4+3]; w4 = w[i4+4]
        ax = cx[a]; ay = cy[a]; az = cz[a]
        bx = cx[b]; by = cy[b]; bz = cz[b]
        ex = cx[cc]; ey = cy[cc]; ez = cz[cc]
        fx2 = cx[d]; fy2 = cy[d]; fz2 = cz[d]
        ax2 = c * ax - s * ay; ay2 = s * ax + c * ay
        bx2 = c * bx - s * by; by2 = s * bx + c * by
        ex2 = c * ex - s * ey; ey2 = s * ex + c * ey
        fx3 = c * fx2 - s * fy2; fy3 = s * fx2 + c * fy2
        px[i] = w2 * bx2 + w3 * ex2 + w4 * fx3 + w1 * ax2
        py[i] = w2 * by2 + w3 * ey2 + w4 * fy3 + w1 * ay2
        pz[i] = w2 * bz + w3 * ez + w4 * fz2 + w1 * az
    end
end

# integer-division probe — probe3 (its CUDA original reads `threadIdx.x`
# without the call parentheses, unlike every other kernel; ported in call form)
@kernel function probe_div!(px, @Const(cx), @Const(idx), nV, ntot)
    i = @index(Global, Linear)
    if i <= ntot
        g = div(i - 1, nV)
        j = i - g * nV
        i4 = 4 * (j - 1)
        a = idx[i4+1]
        px[i] = cx[a]
    end
end

const KERNELS = (deform_corners!, reconstruct!, pair_dev!, fused_deform_reconstruct!,
                 fused_batched_wind!, reconstruct_batched!, rot_direct!, lbs!,
                 morph_lerp!, fused_batched_shared!, probe_div!)

end # module
