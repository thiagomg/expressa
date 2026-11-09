/* Benchmark-only Unix launcher. Keep the parent small so fork/exec accounting
 * cannot inherit the Python driver's much larger RSS high-water mark. */
#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static double seconds(struct timeval value) {
    return (double)value.tv_sec + (double)value.tv_usec / 1000000.0;
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: measure STATS_JSON BINARY PROGRAM [ARGS...]\n");
        return 125;
    }
    struct timespec before, after;
    if (clock_gettime(CLOCK_MONOTONIC, &before) != 0) {
        perror("clock_gettime");
        return 125;
    }
    pid_t child = fork();
    if (child < 0) {
        perror("fork");
        return 125;
    }
    if (child == 0) {
        execv(argv[2], &argv[2]);
        perror("execv");
        _exit(127);
    }
    int status;
    while (waitpid(child, &status, 0) < 0) {
        if (errno != EINTR) {
            perror("waitpid");
            return 125;
        }
    }
    struct rusage usage;
    if (clock_gettime(CLOCK_MONOTONIC, &after) != 0 ||
        getrusage(RUSAGE_CHILDREN, &usage) != 0) {
        perror("resource accounting");
        return 125;
    }
    double wall = (double)(after.tv_sec - before.tv_sec) +
                  (double)(after.tv_nsec - before.tv_nsec) / 1000000000.0;
    double user = seconds(usage.ru_utime), system = seconds(usage.ru_stime);
#ifdef __APPLE__
    unsigned long long rss = (unsigned long long)usage.ru_maxrss;
#else
    unsigned long long rss = (unsigned long long)usage.ru_maxrss * 1024ULL;
#endif
    FILE *file = fopen(argv[1], "w");
    if (!file) {
        perror("stats file");
        return 125;
    }
    int written = fprintf(file,
        "{\"wall_seconds\":%.9f,\"user_seconds\":%.9f,"
        "\"system_seconds\":%.9f,\"cpu_seconds\":%.9f,"
        "\"cpu_percent\":%.6f,\"peak_rss_bytes\":%llu}\n",
        wall, user, system, user + system,
        wall > 0 ? (user + system) / wall * 100.0 : 0.0, rss);
    int closed = fclose(file);
    if (written < 0 || closed != 0) {
        perror("writing stats");
        return 125;
    }
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) return 128 + WTERMSIG(status);
    return 125;
}
