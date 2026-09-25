#import <Cocoa/Cocoa.h>

typedef void (*TerminationCallback)(void);

extern "C" void register_app_will_terminate_handler(TerminationCallback callback) {
    [[NSNotificationCenter defaultCenter]
        addObserverForName:NSApplicationWillTerminateNotification
                    object:nil
                     queue:nil
                usingBlock:^(NSNotification * _Nonnull note) {
                    (void)note;
                    if (callback) {
                        callback();
                    }
                }];
}
