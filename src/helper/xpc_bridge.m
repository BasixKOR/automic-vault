#import <Foundation/Foundation.h>
#import <libproc.h>

@protocol NukeHelperProgressProtocol
- (void)progressEvent:(NSDictionary *)event;
@end

@interface AVPackageSpec : NSObject <NSSecureCoding>
@property(nonatomic, copy) NSString *name;
@property(nonatomic, copy, nullable) NSString *version;
- (instancetype)initWithName:(NSString *)name version:(NSString *_Nullable)version;
- (NSDictionary *)dictionaryValue;
@end

@protocol NukeHelperProtocol
- (void)install:(NSArray<AVPackageSpec *> *)packages
          reply:(void (^)(NSDictionary *result))reply;
- (void)update:(NSArray<AVPackageSpec *> *)packages
         reply:(void (^)(NSDictionary *result))reply;
- (void)uninstall:(NSArray<AVPackageSpec *> *)packages
            reply:(void (^)(NSDictionary *result))reply;
- (void)makeDefault:(NSArray<AVPackageSpec *> *)packages
              reply:(void (^)(NSDictionary *result))reply;
- (void)updateAll:(void (^)(NSDictionary *result))reply;
- (void)installAv:(NSString *)sourcePath
              reply:(void (^)(NSDictionary *result))reply;
- (void)installIsotopeRoot:(NSString *)isotopeName
                     reply:(void (^)(NSDictionary *result))reply;
- (void)convertRadioisotope:(NSString *)isotopeName
                      reply:(void (^)(NSDictionary *result))reply;
- (void)installIsotopeStubs:(NSString *)isotopeName
                      reply:(void (^)(NSDictionary *result))reply;
- (void)rememberIsotopeAlwaysAllow:(NSString *)executablePath
                         scriptPath:(NSString *_Nullable)scriptPath
                      scriptSha256:(NSString *_Nullable)scriptSha256
                               keys:(NSArray<NSString *> *)keys
                              reply:(void (^)(NSDictionary *result))reply;
- (void)refreshRemoteDatabase:(void (^)(BOOL updated))reply;
- (void)checkForUpdates:(void (^)(BOOL hasUpdates))reply;
@end

extern char *nuke_helper_install(
    const char *packages_json,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_update(
    const char *packages_json,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_uninstall(
    const char *packages_json,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_make_default(
    const char *packages_json,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_update_all(
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_install_av(
    const char *source_path,
    const char *caller_path,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_install_isotope_root(
    const char *isotope_name,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_convert_radioisotope(
    const char *isotope_name,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_install_isotope_stubs(
    const char *isotope_name,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern char *nuke_helper_remember_isotope_always_allow(
    const char *executable_path,
    const char *script_path,
    const char *script_sha256,
    const char *keys_json,
    void *context,
    void (*progress_callback)(void *context, const char *event_json));
extern bool nuke_helper_check_for_updates(void);
extern bool nuke_helper_refresh_remote_database(void);
extern void nuke_helper_free_string(char *value);

@interface NukeHelperInvocationContext : NSObject
@property(nonatomic, strong) NSXPCConnection *connection;
@end

@implementation NukeHelperInvocationContext
@end

static void nuke_helper_emit_progress(void *opaque, const char *event_json) {
    NukeHelperInvocationContext *context =
        (__bridge NukeHelperInvocationContext *)opaque;
    if (context == nil || event_json == NULL) {
        return;
    }

    NSData *data = [NSData dataWithBytes:event_json length:strlen(event_json)];
    if (data == nil) {
        return;
    }
    NSDictionary *event = [NSJSONSerialization JSONObjectWithData:data
                                                          options:0
                                                            error:nil];
    if (![event isKindOfClass:[NSDictionary class]]) {
        return;
    }

    id<NukeHelperProgressProtocol> progress =
        [context.connection remoteObjectProxyWithErrorHandler:^(
            NSError *error __unused) {
        }];
    [progress progressEvent:event];
}

static NSString *nuke_helper_caller_executable_path(NSXPCConnection *connection) {
    if (connection == nil) {
        return @"";
    }

    char path_buffer[PROC_PIDPATHINFO_MAXSIZE] = {0};
    int length = proc_pidpath(connection.processIdentifier,
                             path_buffer,
                             sizeof(path_buffer));
    if (length <= 0) {
        return @"";
    }
    return [NSString stringWithUTF8String:path_buffer] ?: @"";
}

@interface NukeHelperService : NSObject <NSXPCListenerDelegate, NukeHelperProtocol>
@property(nonatomic, strong) dispatch_queue_t queue;
@property(nonatomic, assign) NSInteger activeOperations;
@property(nonatomic, strong) NSMutableSet<NSString *> *connections;
@property(nonatomic) dispatch_source_t idleExitTimer;
@end

@implementation AVPackageSpec

+ (BOOL)supportsSecureCoding {
    return YES;
}

- (instancetype)initWithName:(NSString *)name version:(NSString *)version {
    self = [super init];
    if (self == nil) {
        return nil;
    }
    _name = [name copy];
    _version = [version copy];
    return self;
}

- (instancetype)initWithCoder:(NSCoder *)coder {
    NSString *name = [coder decodeObjectOfClass:[NSString class] forKey:@"name"];
    NSString *version =
        [coder decodeObjectOfClass:[NSString class] forKey:@"version"];
    return [self initWithName:name ?: @"" version:version];
}

- (void)encodeWithCoder:(NSCoder *)coder {
    [coder encodeObject:self.name forKey:@"name"];
    [coder encodeObject:self.version forKey:@"version"];
}

- (NSDictionary *)dictionaryValue {
    NSMutableDictionary *dictionary =
        [NSMutableDictionary dictionaryWithObject:self.name ?: @""
                                          forKey:@"name"];
    if (self.version.length > 0) {
        dictionary[@"version"] = self.version;
    } else {
        dictionary[@"version"] = [NSNull null];
    }
    return dictionary;
}

@end

@implementation NukeHelperService

- (instancetype)init {
    self = [super init];
    if (self == nil) {
        return nil;
    }
    _queue = dispatch_queue_create("com.automicvault.nuke-helper.queue",
                                   DISPATCH_QUEUE_SERIAL);
    _connections = [NSMutableSet set];
    return self;
}

- (BOOL)listener:(NSXPCListener *)listener
    shouldAcceptNewConnection:(NSXPCConnection *)newConnection {
    [self cancelIdleExitTimer];
    NSXPCInterface *exported =
        [NSXPCInterface interfaceWithProtocol:@protocol(NukeHelperProtocol)];
    NSSet *packageClasses = [NSSet setWithObjects:[NSArray class],
                                                   [AVPackageSpec class], nil];
    [exported setClasses:packageClasses
             forSelector:@selector(install:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:packageClasses
             forSelector:@selector(update:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:packageClasses
             forSelector:@selector(uninstall:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:packageClasses
             forSelector:@selector(makeDefault:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(install:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(update:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(uninstall:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(makeDefault:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(updateAll:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSString class], nil]
             forSelector:@selector(installAv:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(installAv:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSString class], nil]
             forSelector:@selector(installIsotopeRoot:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(installIsotopeRoot:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSString class], nil]
             forSelector:@selector(convertRadioisotope:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(convertRadioisotope:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSString class], nil]
             forSelector:@selector(installIsotopeStubs:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(installIsotopeStubs:reply:)
           argumentIndex:0
                 ofReply:YES];
    [exported setClasses:[NSSet setWithObjects:[NSString class], nil]
             forSelector:@selector(rememberIsotopeAlwaysAllow:scriptPath:scriptSha256:keys:reply:)
           argumentIndex:0
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSString class], nil]
             forSelector:@selector(rememberIsotopeAlwaysAllow:scriptPath:scriptSha256:keys:reply:)
           argumentIndex:1
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSString class], nil]
             forSelector:@selector(rememberIsotopeAlwaysAllow:scriptPath:scriptSha256:keys:reply:)
           argumentIndex:2
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSArray class],
                                                [NSString class], nil]
             forSelector:@selector(rememberIsotopeAlwaysAllow:scriptPath:scriptSha256:keys:reply:)
           argumentIndex:3
                 ofReply:NO];
    [exported setClasses:[NSSet setWithObjects:[NSDictionary class],
                                                [NSArray class],
                                                [NSString class],
                                                [NSNumber class],
                                                [NSNull class], nil]
             forSelector:@selector(rememberIsotopeAlwaysAllow:scriptPath:scriptSha256:keys:reply:)
           argumentIndex:0
                 ofReply:YES];

    NSXPCInterface *remote =
        [NSXPCInterface interfaceWithProtocol:@protocol(NukeHelperProgressProtocol)];
    [remote setClasses:[NSSet setWithObjects:[NSDictionary class],
                                              [NSArray class],
                                              [NSString class],
                                              [NSNumber class],
                                              [NSNull class], nil]
           forSelector:@selector(progressEvent:)
         argumentIndex:0
               ofReply:NO];

    newConnection.exportedInterface = exported;
    newConnection.exportedObject = self;
    newConnection.remoteObjectInterface = remote;
    NSString *connectionID = NSUUID.UUID.UUIDString;
    @synchronized(self) {
        [self.connections addObject:connectionID];
    }
    __weak typeof(self) weakSelf = self;
    newConnection.invalidationHandler = ^{
      [weakSelf handleConnectionClosed:connectionID];
    };
    newConnection.interruptionHandler = ^{
      [weakSelf handleConnectionClosed:connectionID];
    };
    [newConnection resume];
    return YES;
}

- (void)install:(NSArray<AVPackageSpec *> *)packages
          reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_install([[self serializePackages:packages] UTF8String],
                                   context,
                                   progress_callback);
    }];
}

- (void)update:(NSArray<AVPackageSpec *> *)packages
         reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_update([[self serializePackages:packages] UTF8String],
                                  context,
                                  progress_callback);
    }];
}

- (void)uninstall:(NSArray<AVPackageSpec *> *)packages
            reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_uninstall([[self serializePackages:packages] UTF8String],
                                     context,
                                     progress_callback);
    }];
}

- (void)makeDefault:(NSArray<AVPackageSpec *> *)packages
              reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_make_default([[self serializePackages:packages] UTF8String],
                                        context,
                                        progress_callback);
    }];
}

- (void)updateAll:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_update_all(context, progress_callback);
    }];
}

- (void)installAv:(NSString *)sourcePath
              reply:(void (^)(NSDictionary *result))reply {
    NSXPCConnection *connection = NSXPCConnection.currentConnection;
    NSString *callerPath = nuke_helper_caller_executable_path(connection);
    [self executeWithConnection:connection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_install_av(sourcePath.UTF8String,
                                      callerPath.UTF8String,
                                      context,
                                      progress_callback);
    }];
}

- (void)installIsotopeRoot:(NSString *)isotopeName
                     reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_install_isotope_root(isotopeName.UTF8String,
                                                context,
                                                progress_callback);
    }];
}

- (void)convertRadioisotope:(NSString *)isotopeName
                      reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_convert_radioisotope(isotopeName.UTF8String,
                                                context,
                                                progress_callback);
    }];
}

- (void)installIsotopeStubs:(NSString *)isotopeName
                      reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_install_isotope_stubs(isotopeName.UTF8String,
                                                 context,
                                                 progress_callback);
    }];
}

- (void)rememberIsotopeAlwaysAllow:(NSString *)executablePath
                         scriptPath:(NSString *_Nullable)scriptPath
                      scriptSha256:(NSString *_Nullable)scriptSha256
                               keys:(NSArray<NSString *> *)keys
                              reply:(void (^)(NSDictionary *result))reply {
    [self executeWithConnection:NSXPCConnection.currentConnection
                         reply:reply
                          body:^char *(void *context, void (*progress_callback)(
                                                      void *, const char *)) {
        return nuke_helper_remember_isotope_always_allow(
            executablePath.UTF8String,
            scriptPath.UTF8String,
            scriptSha256.UTF8String,
            [[self serializeStringArray:keys] UTF8String],
            context,
            progress_callback);
    }];
}

- (void)checkForUpdates:(void (^)(BOOL hasUpdates))reply {
    dispatch_async(self.queue, ^{
      reply(nuke_helper_check_for_updates());
    });
}

- (void)refreshRemoteDatabase:(void (^)(BOOL updated))reply {
    dispatch_async(self.queue, ^{
      reply(nuke_helper_refresh_remote_database());
    });
}

- (NSString *)serializePackages:(NSArray<AVPackageSpec *> *)packages {
    NSMutableArray *values = [NSMutableArray arrayWithCapacity:packages.count];
    for (AVPackageSpec *package in packages) {
        [values addObject:[package dictionaryValue]];
    }
    NSData *data = [NSJSONSerialization dataWithJSONObject:values
                                                   options:0
                                                     error:nil];
    if (data == nil) {
        return @"[]";
    }
    return [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding] ?:
        @"[]";
}

- (NSString *)serializeStringArray:(NSArray<NSString *> *)values {
    NSData *data = [NSJSONSerialization dataWithJSONObject:values ?: @[]
                                                   options:0
                                                     error:nil];
    if (data == nil) {
        return @"[]";
    }
    return [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding] ?:
        @"[]";
}

- (void)executeWithConnection:(NSXPCConnection *)connection
                        reply:(void (^)(NSDictionary *result))reply
                         body:(char * (^)(void *context, void (*progress_callback)(
                                                void *, const char *)))body {
    dispatch_async(self.queue, ^{
      self.activeOperations += 1;
      [self cancelIdleExitTimer];
      NukeHelperInvocationContext *context = [NukeHelperInvocationContext new];
      context.connection = connection;
      char *result_json =
          body((__bridge void *)context, nuke_helper_emit_progress);
      NSDictionary *result = [self parseResultJSON:result_json];
      if (result_json != NULL) {
          nuke_helper_free_string(result_json);
      }
      reply(result ?: @{
          @"Err" : @"helper returned an invalid response",
      });
      self.activeOperations -= 1;
      [self scheduleIdleExitIfNeeded];
    });
}

- (NSDictionary *)parseResultJSON:(const char *)result_json {
    if (result_json == NULL) {
        return nil;
    }
    NSData *data = [NSData dataWithBytes:result_json length:strlen(result_json)];
    if (data == nil) {
        return nil;
    }
    NSDictionary *result = [NSJSONSerialization JSONObjectWithData:data
                                                           options:0
                                                             error:nil];
    if (![result isKindOfClass:[NSDictionary class]]) {
        return nil;
    }
    return result;
}

- (void)handleConnectionClosed:(NSString *)connectionID {
    dispatch_async(self.queue, ^{
      @synchronized(self) {
          [self.connections removeObject:connectionID];
      }
      [self scheduleIdleExitIfNeeded];
    });
}

- (void)cancelIdleExitTimer {
    if (self.idleExitTimer == nil) {
        return;
    }
    dispatch_source_cancel(self.idleExitTimer);
    self.idleExitTimer = nil;
}

- (void)scheduleIdleExitIfNeeded {
    if (self.activeOperations > 0) {
        return;
    }
    @synchronized(self) {
        if (self.connections.count > 0) {
            return;
        }
    }
    if (self.idleExitTimer != nil) {
        return;
    }
    dispatch_source_t timer =
        dispatch_source_create(DISPATCH_SOURCE_TYPE_TIMER, 0, 0, self.queue);
    self.idleExitTimer = timer;
    dispatch_source_set_timer(
        timer,
        dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC),
        DISPATCH_TIME_FOREVER,
        250 * NSEC_PER_MSEC);
    __weak typeof(self) weakSelf = self;
    dispatch_source_set_event_handler(timer, ^{
      NukeHelperService *strongSelf = weakSelf;
      if (strongSelf == nil || strongSelf.idleExitTimer != timer) {
          return;
      }
      @synchronized(strongSelf) {
          if (strongSelf.connections.count > 0 ||
              strongSelf.activeOperations > 0) {
              [strongSelf cancelIdleExitTimer];
              return;
          }
      }
      exit(EXIT_SUCCESS);
    });
    dispatch_resume(timer);
}

@end

void nuke_helper_run_service(void) {
    @autoreleasepool {
        NukeHelperService *delegate = [NukeHelperService new];
        NSXPCListener *listener = [[NSXPCListener alloc]
            initWithMachServiceName:@"com.automicvault.nuke-helper"];
        listener.delegate = delegate;
        [listener resume];
        [[NSRunLoop currentRunLoop] run];
    }
}
