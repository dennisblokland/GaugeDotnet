using SkiaSharp;
using SDL2;

namespace GaugeDotnet
{
    public class GaugeSDL
    {
        private IntPtr _window;
        private IntPtr _glContext;
        private GRContext? _grContext;
        private SKSurface? _skSurface;


        public GaugeSDL(int screenWidth, int screenHeight)
        {
            // 1) Initialize SDL video
            if (SDL.SDL_Init(SDL.SDL_INIT_VIDEO) < 0)
            {
                Console.WriteLine($"SDL could not initialize! SDL_Error: {SDL.SDL_GetError()}");
                return;
            }

            // 2) Tell SDL we want an OpenGL ES 2.0 context
            SDL.SDL_GL_SetAttribute(
                SDL.SDL_GLattr.SDL_GL_CONTEXT_PROFILE_MASK,
                (int)SDL.SDL_GLprofile.SDL_GL_CONTEXT_PROFILE_ES);
            SDL.SDL_GL_SetAttribute(SDL.SDL_GLattr.SDL_GL_CONTEXT_MAJOR_VERSION, 2);
            SDL.SDL_GL_SetAttribute(SDL.SDL_GLattr.SDL_GL_CONTEXT_MINOR_VERSION, 0);

            // 3) Create an SDL window with OPENGL
            _window = SDL.SDL_CreateWindow(
                "SkiaSharp + SDL2 Window",
                SDL.SDL_WINDOWPOS_CENTERED, SDL.SDL_WINDOWPOS_CENTERED,
                screenWidth, screenHeight,
                SDL.SDL_WindowFlags.SDL_WINDOW_OPENGL
              | SDL.SDL_WindowFlags.SDL_WINDOW_SHOWN);

            if (_window == IntPtr.Zero)
            {
                Console.WriteLine($"Window could not be created! SDL_Error: {SDL.SDL_GetError()}");
                SDL.SDL_Quit();
                return;
            }

            // 4) Create GL context
            _glContext = SDL.SDL_GL_CreateContext(_window);
            if (_glContext == IntPtr.Zero)
            {
                Console.WriteLine($"SDL_GL_CreateContext failed: {SDL.SDL_GetError()}");
                Cleanup();
                throw new InvalidOperationException("Failed to create OpenGL context.");
            }
            SDL.SDL_GL_MakeCurrent(_window, _glContext);

            // (Optional) Enable v-sync
            SDL.SDL_GL_SetSwapInterval(1);

            try
            {
                // 5) Initialize Skia GPU context
                GRGlInterface glInterface =  GRGlInterface.Create() ?? GRGlInterface.CreateGles(SDL.SDL_GL_GetProcAddress);
                if (glInterface == null)
                {
                    Cleanup();
                    throw new InvalidOperationException("Failed to create SkiaSharp GRGlInterface for OpenGL ES.");
                }
                _grContext = GRContext.CreateGl(glInterface);
                if (_grContext == null)
                {
                    Cleanup();
                    throw new InvalidOperationException("Failed to create SkiaSharp GRContext for OpenGL.");
                }

                Console.WriteLine(_grContext.Backend);

                // 6) Create a SkiaSurface that targets FBO #0 (the window's backbuffer)
                GRGlFramebufferInfo glFramebufferInfo = new(
                    /*fboId=*/ 0u,
                    /*format=*/ SKColorType.Rgba8888.ToGlSizedFormat()
                );

                GRBackendRenderTarget backendRT = new(
                    screenWidth,
                    screenHeight,
                    /*sampleCount=*/ 0,
                    /*stencilBits=*/ 8,
                    glFramebufferInfo
                );

                _skSurface = SKSurface.Create(
                    _grContext,
                    backendRT,
                    GRSurfaceOrigin.BottomLeft,
                    SKColorType.Rgba8888
                );
                if (_skSurface == null)
                {
                    Console.WriteLine("Failed to create SKSurface for SDL window.");
                    Cleanup();
                    return;
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"Error initializing SkiaSharp: {ex.Message}");
                Cleanup();
                throw;
            }
        }


        private void Cleanup()
        {
            _skSurface?.Dispose();
            _grContext?.Dispose();

            if (_glContext != IntPtr.Zero)
            {
                SDL.SDL_GL_DeleteContext(_glContext);
                _glContext = IntPtr.Zero;
            }
            if (_window != IntPtr.Zero)
            {
                SDL.SDL_DestroyWindow(_window);
                _window = IntPtr.Zero;
            }
            SDL.SDL_Quit();
        }

        internal void FlushAndSwap()
        {
            // Flush Skia → GL
            _skSurface?.Flush();
            _grContext?.Flush();
            // Swap the SDL window buffers
            SDL.SDL_GL_SwapWindow(_window);
        }

        internal SKCanvas GetCanvas()
        {
            if (_skSurface == null)
            {
                throw new InvalidOperationException("SKSurface is not initialized.");
            }
            return _skSurface.Canvas;
        }
    }
}
