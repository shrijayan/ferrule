# typed: strong

module __FerruleModule__
  module Internal
    module Type
      # @api private
      #
      # This module provides a base implementation for paginated responses in the SDK.
      module BasePage
        Elem = type_member(:out)

        sig { overridable.returns(T::Boolean) }
        def next_page?
        end

        sig { overridable.returns(T.self_type) }
        def next_page
        end

        sig { overridable.params(blk: T.proc.params(arg0: Elem).void).void }
        def auto_paging_each(&blk)
        end

        sig { returns(T::Enumerable[Elem]) }
        def to_enum
        end

        # @api private
        sig do
          params(
            client: __FerruleModule__::Internal::Transport::BaseClient,
#ferrule:if fits
            req: __FerruleModule__::Internal::Transport::BaseClient::RequestComponents,
#ferrule:else
            req:
              __FerruleModule__::Internal::Transport::BaseClient::RequestComponents,
#ferrule:endif
            headers: T::Hash[String, String],
            page_data: T.anything
          ).void
        end
        def initialize(client:, req:, headers:, page_data:)
        end
      end
    end
  end
end
