# typed: strong

module __FerruleModule__
  module Internal
    module Type
      # @api private
      module RequestParameters
        # Options to specify HTTP behaviour for this request.
        sig { returns(__FerruleModule__::RequestOptions) }
        attr_reader :request_options

#ferrule:if fits
        sig { params(request_options: __FerruleModule__::RequestOptions::OrHash).void }
#ferrule:else
        sig do
          params(request_options: __FerruleModule__::RequestOptions::OrHash).void
        end
#ferrule:endif
        attr_writer :request_options

        # @api private
        module Converter
          # @api private
          sig do
            params(params: T.anything).returns(
              [T.anything, __FerruleModule__::Internal::AnyHash]
            )
          end
          def dump_request(params)
          end
        end
      end
    end
  end
end
