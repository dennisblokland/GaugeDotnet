﻿using System.Collections;
using System.Diagnostics;
using System.Runtime.InteropServices;

namespace RG35XX.Libraries
{
    public static class Utilities
    {
        public static void Reboot()
        {
            Run("reboot -f");
        }

        internal static void Run(string command)
        {
            // Properly escape the command for bash -c
            string escapedCommand = command
                .Replace("\"", "\\\"")     // Escape double quotes
                .Replace("$", "\\$");

            ProcessStartInfo startInfo = new()
            {
                FileName = "/bin/bash",
                WorkingDirectory = AppContext.BaseDirectory,
                Arguments = $"-c \"{command}\"",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            // Copy existing environment variables
            foreach (DictionaryEntry env in Environment.GetEnvironmentVariables())
            {
                startInfo.Environment[env.Key.ToString()] = env.Value.ToString();
            }

            using Process process = new()
            {
                StartInfo = startInfo
            };

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                process.Start();

                // Start asynchronous read operations
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();

                process.WaitForExit();
            }
        }

        public static async Task<bool> CorrectSystemTime()
        {
#if DEBUG
            return false;
#endif

            int maxRetries = 3;
            int currentRetry = 0;
            int backoffMs = 1000; // Start with 1 second

            while (currentRetry < maxRetries)
            {
                try
                {
                    // Create handler that ignores SSL validation
                    using HttpClientHandler handler = new()
                    {
                        ServerCertificateCustomValidationCallback = (message, cert, chain, errors) => true
                    };

                    // Create and configure client
                    using HttpClient client = new(handler);
                    client.DefaultRequestHeaders.Add("User-Agent", "Mozilla/5.0");

                    // Use HEAD request to minimize data transfer
                    using HttpRequestMessage request = new(HttpMethod.Head, "http://google.com");
                    using HttpResponseMessage response = await client.SendAsync(request);

                    if (response.Headers.Date.HasValue)
                    {
                        string dateCommand = response.Headers.Date.Value.UtcDateTime.ToString("MMddHHmmyyyy.ss");
                        Run($"date {dateCommand}");
                        return true; // Success - exit method
                    }
                    else
                    {
                        return false; // No date header - exit method
                    }
                }
                catch (Exception ex)
                {
                    currentRetry++;
                    if (currentRetry < maxRetries)
                    {
                        await Task.Delay(backoffMs);
                        backoffMs *= 2; // Exponential backoff
                    }
                    else
                    {
                        return false;
                    }
                }
            }

            return false;
        }
    }
}